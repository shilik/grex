/*
 * Copyright © 2019 Peter M. Stahl pemistahl@gmail.com
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either expressed or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use crate::ast::Expression;
use crate::dfa::DFA;
use crate::grapheme::GraphemeCluster;
use itertools::Itertools;
use std::clone::Clone;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter, Result};

pub struct RegExpBuilder {
    test_cases: Vec<String>,
    is_non_ascii_char_escaped: bool,
    is_astral_code_point_converted_to_surrogate: bool,
    is_repetition_converted: bool,
}

impl RegExpBuilder {
    pub fn from<T: Clone + Into<String>>(test_cases: &[T]) -> Self {
        Self {
            test_cases: test_cases.iter().cloned().map(|it| it.into()).collect_vec(),
            is_non_ascii_char_escaped: false,
            is_astral_code_point_converted_to_surrogate: false,
            is_repetition_converted: false,
        }
    }

    pub fn with_escaped_non_ascii_chars(&mut self, use_surrogate_pairs: bool) -> &mut Self {
        self.is_non_ascii_char_escaped = true;
        self.is_astral_code_point_converted_to_surrogate = use_surrogate_pairs;
        self
    }

    pub fn with_converted_repetitions(&mut self) -> &mut Self {
        self.is_repetition_converted = true;
        self
    }

    pub fn build(&mut self) -> String {
        RegExp::from(
            &mut self.test_cases,
            self.is_non_ascii_char_escaped,
            self.is_astral_code_point_converted_to_surrogate,
            self.is_repetition_converted,
        )
        .to_string()
    }
}

pub(crate) struct RegExp {
    ast: Expression,
}

impl RegExp {
    fn from(
        test_cases: &mut Vec<String>,
        is_non_ascii_char_escaped: bool,
        is_astral_code_point_converted_to_surrogate: bool,
        is_repetition_converted: bool,
    ) -> Self {
        Self::sort(test_cases);
        Self {
            ast: Expression::from(DFA::from(Self::grapheme_clusters(
                &test_cases,
                is_non_ascii_char_escaped,
                is_astral_code_point_converted_to_surrogate,
                is_repetition_converted,
            ))),
        }
    }

    fn sort(test_cases: &mut Vec<String>) {
        test_cases.sort();
        test_cases.dedup();
        test_cases.sort_by(|a, b| match a.len().cmp(&b.len()) {
            Ordering::Equal => a.cmp(&b),
            other => other,
        });
    }

    fn grapheme_clusters(
        test_cases: &[String],
        is_non_ascii_char_escaped: bool,
        is_astral_code_point_converted_to_surrogate: bool,
        is_repetition_converted: bool,
    ) -> Vec<GraphemeCluster> {
        let mut clusters = test_cases
            .iter()
            .map(|it| GraphemeCluster::from(it))
            .collect_vec();

        if is_repetition_converted {
            for cluster in clusters.iter_mut() {
                cluster.convert_repetitions();
            }
        }

        if is_non_ascii_char_escaped {
            for cluster in clusters.iter_mut() {
                cluster.escape_non_ascii_chars(is_astral_code_point_converted_to_surrogate);
            }
        }

        clusters
    }
}

impl Display for RegExp {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "^{}$", self.ast.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;
    use rstest::rstest;

    #[rstest(test_cases, expected_output,
        case(vec![], "^$"),
        case(vec![""], "^$"),
        case(vec![" "], "^ $"),
        case(vec!["   "], "^   $"),
        case(vec!["a", "b"], "^[ab]$"),
        case(vec!["a", "b", "c"], "^[a-c]$"),
        case(vec!["a", "c", "d", "e", "f"], "^[ac-f]$"),
        case(vec!["a", "b", "x", "d", "e"], "^[abdex]$"),
        case(vec!["a", "b", "x", "de"], "^de|[abx]$"),
        case(vec!["a", "b", "c", "x", "d", "e"], "^[a-ex]$"),
        case(vec!["a", "b", "c", "x", "de"], "^de|[a-cx]$"),
        case(vec!["a", "b", "c", "d", "e", "f", "o", "x", "y", "z"], "^[a-fox-z]$"),
        case(vec!["a", "b", "d", "e", "f", "o", "x", "y", "z"], "^[abd-fox-z]$"),
        case(vec!["1", "2"], "^[12]$"),
        case(vec!["1", "2", "3"], "^[1-3]$"),
        case(vec!["1", "3", "4", "5", "6"], "^[13-6]$"),
        case(vec!["1", "2", "8", "4", "5"], "^[12458]$"),
        case(vec!["1", "2", "8", "45"], "^45|[128]$"),
        case(vec!["1", "2", "3", "8", "4", "5"], "^[1-58]$"),
        case(vec!["1", "2", "3", "8", "45"], "^45|[1-38]$"),
        case(vec!["1", "2", "3", "5", "7", "8", "9"], "^[1-357-9]$"),
        case(vec!["a", "b", "bc"], "^bc?|a$"),
        case(vec!["a", "b", "bcd"], "^b(cd)?|a$"),
        case(vec!["a", "ab", "abc"], "^a(bc?)?$"),
        case(vec!["ac", "bc"], "^[ab]c$"),
        case(vec!["ab", "ac"], "^a[bc]$"),
        case(vec!["abx", "cdx"], "^(ab|cd)x$"),
        case(vec!["abd", "acd"], "^a[bc]d$"),
        case(vec!["abc", "abcd"], "^abcd?$"),
        case(vec!["abc", "abcde"], "^abc(de)?$"),
        case(vec!["ade", "abcde"], "^a(bc)?de$"),
        case(vec!["abcxy", "adexy"], "^a(bc|de)xy$"),
        case(vec!["axy", "abcxy", "adexy"], "^a((bc)?|de)xy$"), // goal: "^a(bc|de)?xy$"
        case(vec!["abcxy", "abcw", "efgh"], "^abc(xy|w)|efgh$"),
        case(vec!["abcxy", "efgh", "abcw"], "^abc(xy|w)|efgh$"),
        case(vec!["efgh", "abcxy", "abcw"], "^abc(xy|w)|efgh$"),
        case(vec!["abxy", "cxy", "efgh"], "^(ab|c)xy|efgh$"),
        case(vec!["abxy", "efgh", "cxy"], "^(ab|c)xy|efgh$"),
        case(vec!["efgh", "abxy", "cxy"], "^(ab|c)xy|efgh$"),
        case(vec!["a", "ä", "o", "ö", "u", "ü"], "^[aouäöü]$"),
        case(vec!["y̆", "a", "z"], "^[az]|y̆$"), // goal: "^[az]|y\\u{306}$"
        case(vec!["a", "b\n", "c"], "^b\\n|[ac]$"),
        case(vec!["a", "b\\n", "c"], "^b\\\\n|[ac]$"),
        case(vec!["[a-z]", "(d,e,f)"], "^\\(d,e,f\\)|\\[a\\-z\\]$"),
        case(vec!["3.5", "4.5", "4,5"], "^3\\.5|4[,.]5$"),
        case(vec!["I ♥ cake"], "^I ♥ cake$"),
        case(vec!["I \u{2665} cake"], "^I ♥ cake$"),
        case(vec!["I \\u{2665} cake"], "^I \\\\u\\{2665\\} cake$"),
        case(vec!["I \\u2665 cake"], "^I \\\\u2665 cake$"),
        case(vec!["My ♥ is yours.", "My 💩 is yours."], "^My [♥💩] is yours\\.$")
    )]
    fn regexp_builder_with_default_settings(test_cases: Vec<&str>, expected_output: &str) {
        let regexp = RegExpBuilder::from(&test_cases).build();
        test_if_regexp_is_correct(regexp, expected_output);
        test_if_regexp_matches_test_cases(expected_output, test_cases);
    }

    #[rstest(test_cases, expected_output,
        case(vec![], "^$"),
        case(vec![""], "^$"),
        case(vec![" "], "^ $"),
        case(vec!["   "], "^ {3}$"),
        case(vec!["a"], "^a$"),
        case(vec!["aa"], "^a{2}$"),
        case(vec!["aaa"], "^a{3}$"),
        case(vec!["a", "aa"], "^a{1,2}$"),
        case(vec!["aaa", "a", "aa"], "^a{1,3}$"),
        case(vec!["aaaa", "a", "aa"], "^a{1,2}|a{4}$"),
        case(vec!["ababab"], "^(ab){3}$"),
        case(vec!["abababa"], "^(ab){3}a$"),
        case(vec!["aababab"], "^a(ab){3}$"),
        case(vec!["abababaa"], "^(ab){3}a{2}$"),
        case(vec!["b", "ba"], "^ba?$"),
        case(vec!["b", "ba", "baa"], "^b(a{1,2})?$"),
        case(vec!["b", "ba", "baaa", "baa"], "^b(a{1,3})?$"),
        case(vec!["b", "ba", "baaaa", "baa"], "^b(a{1,2}|a{4})?$"),
        case(vec!["axy", "abcxyxy", "adexy"], "^a((de)?xy|bc(xy){2})$"),
        case(vec!["xy̆y̆y̆y̆z"], "^x(y̆){4}z$"),
        case(vec!["xy̆y̆z", "xy̆y̆y̆z"], "^x(y̆){2,3}z$"),
        case(vec!["xy̆y̆z", "xy̆y̆y̆y̆z"], "^x((y̆){2}|(y̆){4})z$"),
        case(vec!["zyxx", "yxx"], "^z?yx{2}$"),
        case(vec!["zyxx", "yxx", "yxxx"], "^zyx{2}|yx{2,3}$"),
        case(vec!["zyxxx", "yxx", "yxxx"], "^zyx{3}|yx{2,3}$"),
        case(vec!["a", "b\n\n", "c"], "^b\\n{2}|[ac]$"),
        case(vec!["a", "b\nb\nb", "c"], "^(b\\n){2}b|[ac]$"),
        case(vec!["a", "b\nx\nx", "c"], "^b(\\nx){2}|[ac]$"),
        case(vec!["a", "b\n\t\n\t", "c"], "^b(\\n\\t){2}|[ac]$"),
        case(vec!["a", "b\n", "b\n\n", "b\n\n\n", "c"], "^b\\n{1,3}|[ac]$"),
        case(vec!["4.5", "3.55"], "^4\\.5|3\\.5{2}$"),
        case(vec!["4.5", "4.55"], "^4\\.5{1,2}$"),
        case(vec!["4.5", "4.55", "3.5"], "^3\\.5|4\\.5{1,2}$"),
        case(vec!["4.5", "44.5", "44.55", "4.55"], "^4{1,2}\\.5{1,2}$"),
        case(vec!["I ♥♥ cake"], "^I ♥{2} cake$"),
        case(vec!["I ♥ cake", "I ♥♥ cake"], "^I ♥{1,2} cake$"),
        case(vec!["I \u{2665}\u{2665} cake"], "^I ♥{2} cake$"),
        case(vec!["I \\u{2665} cake"], "^I \\\\u\\{26{2}5\\} cake$"),
        case(vec!["I \\u{2665}\\u{2665} cake"], "^I (\\\\u\\{2665\\}){2} cake$"),
        case(vec!["I \\u2665\\u2665 cake"], "^I (\\\\u2665){2} cake$"),
        case(vec!["My ♥♥♥ is yours.", "My 💩💩 is yours."], "^My (💩{2}|♥{3}) is yours\\.$")
    )]
    fn regexp_builder_with_converted_repetitions(test_cases: Vec<&str>, expected_output: &str) {
        let regexp = RegExpBuilder::from(&test_cases)
            .with_converted_repetitions()
            .build();
        test_if_regexp_is_correct(regexp, expected_output);
        test_if_regexp_matches_test_cases(expected_output, test_cases);
    }

    #[rstest(test_cases, expected_output,
        case(vec!["My ♥ and 💩 is yours."], "^My \\u{2665} and \\u{1f4a9} is yours\\.$"),
        case(vec!["My ♥ is yours.", "My 💩 is yours."], "^My (\\u{2665}|\\u{1f4a9}) is yours\\.$")
    )]
    fn regexp_builder_with_escaped_non_ascii_chars(test_cases: Vec<&str>, expected_output: &str) {
        let regexp = RegExpBuilder::from(&test_cases)
            .with_escaped_non_ascii_chars(false)
            .build();
        test_if_regexp_is_correct(regexp, expected_output);
        test_if_regexp_matches_test_cases(expected_output, test_cases);
    }

    #[rstest(test_cases, expected_output,
        case(vec!["My ♥ and 💩 is yours."], "^My \\u{2665} and \\u{d83d}\\u{dca9} is yours\\.$"),
        case(vec!["My ♥ is yours.", "My 💩 is yours."], "^My (\\u{2665}|\\u{d83d}\\u{dca9}) is yours\\.$")
    )]
    fn regexp_builder_with_escaped_non_ascii_chars_and_surrogates(
        test_cases: Vec<&str>,
        expected_output: &str,
    ) {
        let regexp = RegExpBuilder::from(&test_cases)
            .with_escaped_non_ascii_chars(true)
            .build();
        test_if_regexp_is_correct(regexp, expected_output);
    }

    #[rstest(test_cases, expected_output,
        case(vec!["My ♥♥♥ and 💩💩 is yours."], "^My \\u{2665}{3} and \\u{1f4a9}{2} is yours\\.$"),
        case(vec!["My ♥♥♥ is yours.", "My 💩💩 is yours."], "^My (\\u{1f4a9}{2}|\\u{2665}{3}) is yours\\.$")
    )]
    fn regexp_builder_with_converted_repetitions_and_escaped_chars(
        test_cases: Vec<&str>,
        expected_output: &str,
    ) {
        let regexp = RegExpBuilder::from(&test_cases)
            .with_converted_repetitions()
            .with_escaped_non_ascii_chars(false)
            .build();
        test_if_regexp_is_correct(regexp, expected_output);
        test_if_regexp_matches_test_cases(expected_output, test_cases);
    }

    #[rstest(test_cases, expected_output,
        case(vec!["My ♥♥♥ and 💩💩 is yours."], "^My \\u{2665}{3} and (\\u{d83d}\\u{dca9}){2} is yours\\.$"),
        case(vec!["My ♥♥♥ is yours.", "My 💩💩 is yours."], "^My ((\\u{d83d}\\u{dca9}){2}|\\u{2665}{3}) is yours\\.$")
    )]
    fn regexp_builder_with_converted_repetitions_and_escaped_chars_and_surrogates(
        test_cases: Vec<&str>,
        expected_output: &str,
    ) {
        let regexp = RegExpBuilder::from(&test_cases)
            .with_converted_repetitions()
            .with_escaped_non_ascii_chars(true)
            .build();
        test_if_regexp_is_correct(regexp, expected_output);
    }

    fn test_if_regexp_is_correct(regexp: String, expected_output: &str) {
        assert_eq!(regexp, expected_output);
    }

    fn test_if_regexp_matches_test_cases(expected_output: &str, test_cases: Vec<&str>) {
        let re = Regex::new(expected_output).unwrap();
        for test_case in test_cases {
            assert!(
                re.is_match(test_case),
                "\"{}\" does not match regex",
                test_case
            );
        }
    }
}
