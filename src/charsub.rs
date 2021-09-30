//! This module provides the implementtion of the finl CharSub mechanism. An input string will be
//! read and returned as a `Cow<str>` which either has the unmodified original string or a new
//! string with any substitutions applied.
//!
//! So, for example, given the substitutions:
//!
//!  ```text
//!  `` → “
//!  '' → ”
//!  ````
//!
//! The input string
//!
//! ```text
//! This is ``amazing''
//! ```
//!
//! will give the output string
//!
//! ```text
//! This is “amazing”
//! ```
//!

use anyhow;
use std::borrow::Cow;
use std::collections::HashMap;
use thiserror::Error;
use std::io::{BufRead};

/// The implementation of a char substitution machine. This is a non-thread-safe implementation with
/// mutable state.
pub struct CharSubMachine {
    trie: SubstitutionTrie,
    unprocessed: Option<String>,
}

impl CharSubMachine {
    /// Create a new blank char substitution machine.
    pub fn new() -> CharSubMachine {
        CharSubMachine {
            trie: SubstitutionTrie::new(),
            unprocessed: None,
        }
    }

    /// Create a new blank char substitution machine instantiated from the specified file (given
    /// as a `std::io::buffered::BufReader`. Since errors might occur during the read process, the result
    /// is wrapped in `anyhow::Result`. Errors can be of type `CharSubError`, `UnescapeError` or `io::Error`
    /// depending on the origin of the error.
    pub fn from_buf_reader<R: BufRead>(input: &mut R) -> anyhow::Result<CharSubMachine> {
        let mut return_value = CharSubMachine::new();
        for result in input.lines()
                .map(|l| {
                    parse_charsub_config_line(l?.as_str())
                })
                .filter_map(|x| x.transpose()) {
            let (from, to) = result?;
            return_value.add_substitution(from.as_str(), to.as_str());
        }
        Ok(return_value)
    }

    /// Add a new substitution rule to an existing char sub machine. This will not fail.
    pub fn add_substitution(&mut self, input: &str, output: &str) {
        self.trie.add(input, output);
    }

    /// Process an input string and return the substitution in `alloc::borrow::Cow<str>`. This will
    /// not fail, but may leave unprocessed output behind if the end of the string might be part
    /// pf a longer substitution, e.g..
    /// ```
    /// # use finl_charsub::charsub::CharSubMachine;
    /// let mut char_sub_machine = CharSubMachine::new();
    ///
    /// char_sub_machine.add_substitution("A", "a");
    /// char_sub_machine.add_substitution("ABC", "b");
    ///
    /// assert_eq!("", char_sub_machine.process("AB"));
    /// assert_eq!("b", char_sub_machine.process("C"));
    /// ```
    pub fn process<'a>(&mut self, input: &'a str) -> Cow<'a, str> {
        let mut curr_node = &self.trie;
        let mut built_value: Option<String> = None;
        let mut in_substitution = false;
        let mut substitution_start = 0;
        let no_change = Cow::Borrowed(input);
        let mut new_input;
        let input = match &self.unprocessed.take() {
            None => input,
            Some(unprocessed) => {
                new_input = String::with_capacity(unprocessed.len() + input.len());
                new_input.push_str(unprocessed.as_str());
                new_input.push_str(input);
                built_value = Some(String::with_capacity(new_input.len()));
                new_input.as_str()
            }
        };
        for (loc, ch) in input.char_indices() {
            if in_substitution && !curr_node.children.contains_key(&ch) {
                if in_substitution {
                    match &curr_node.output {
                        None => {
                            let (substitution, remainder) = self
                                ._flush_substitution(input.get(substitution_start..loc).unwrap());
                            built_value
                                .as_mut()
                                .unwrap()
                                .push_str(substitution.as_str());
                            if let Some(remainder) = remainder {
                                built_value.as_mut().unwrap().push_str(remainder.as_str());
                            }
                        }
                        Some(substitution) => {
                            built_value
                                .as_mut()
                                .unwrap()
                                .push_str(substitution.as_str());
                        }
                    }
                    in_substitution = false;
                    curr_node = &self.trie;
                }
            }
            if curr_node.children.contains_key(&ch) {
                if built_value.is_none() {
                    built_value = Some(String::with_capacity(input.len()));
                    built_value
                        .as_mut()
                        .unwrap()
                        .push_str(input.get(..loc).unwrap());
                }
                if !in_substitution {
                    substitution_start = loc;
                }
                in_substitution = true;
                curr_node = curr_node.children.get(&ch).unwrap();
            } else {
                if let Some(output) = &mut built_value {
                    output.push(ch);
                }
            }
        }
        if in_substitution {
            if curr_node.children.is_empty() {
                built_value
                    .as_mut()
                    .unwrap()
                    .push_str(&curr_node.output.as_ref().unwrap().as_str());
            } else {
                self.unprocessed = input.get(substitution_start..).map(|s| s.to_string());
            }
        }
        match built_value {
            None => no_change,
            Some(_) => Cow::Owned(built_value.unwrap()),
        }
    }

    /// Returns a possibly empty string with the contents of any unprocessed
    /// input still waiting in the unprocessed buffer.
    /// ```
    /// # use finl_charsub::charsub::CharSubMachine;
    /// let mut char_sub_machine = CharSubMachine::new();
    ///
    /// char_sub_machine.add_substitution("A", "a");
    /// char_sub_machine.add_substitution("ABC", "b");
    ///
    /// assert_eq!("", char_sub_machine.process("AB"));
    /// assert_eq!("aB".to_string(), char_sub_machine.flush());
    ///
    /// assert_eq!("b", char_sub_machine.process("ABC"));
    /// assert_eq!("".to_string(), char_sub_machine.flush());
    /// ```
    pub fn flush(&mut self) -> String {
        match &self.unprocessed.take() {
            None => String::new(),
            Some(unprocessed) => {
                let (rv_part1, rv_part2) = self._flush_substitution(unprocessed.as_str());
                let mut rv_part1 = rv_part1.clone();
                if rv_part2.is_some() {
                    rv_part1.push_str(rv_part2.unwrap().as_str());
                }
                rv_part1
            }
        }
    }

    // Private method which takes an unevaluated string and returns the substitution along with—
    // possibly—the remainder of the string. This gets called both by `flush()` but also in the context
    // of a dead-end path in the substitution process, e.g., given substitutions ABC -> d, A -> c
    // then the input ABQ should return cBQ, which requires backtracking to the A when the possible
    // ABC substitution does not give the expected result.
    fn _flush_substitution(&mut self, input: &str) -> (String, Option<String>) {
        let mut curr_node = &self.trie;
        let mut end_of_mapping = 0;
        let mut in_substitution = false;
        for (loc, ch) in input.char_indices() {
            // We have already looked at this sequence, so we know that every character in input
            // is mapped in the trie
            curr_node = curr_node.children.get(&ch).unwrap();
            if curr_node.output.is_some() {
                in_substitution = true;
            }
            else if in_substitution {
                end_of_mapping = loc;
                in_substitution = false;
            }
        }
        if in_substitution {
            end_of_mapping = input.len();
        }
        match &curr_node.output {
            None => {
                if end_of_mapping == 0 {
                    (input.to_string(), None)
                } else {
                    (
                        self._flush_substitution(input.get(..end_of_mapping).unwrap())
                            .0,
                        input.get(end_of_mapping..).map(|s| s.to_string()),
                    )
                }
            }
            Some(output) => (output.clone(), None),
        }
    }
}

/// Errors from reading the char sub file.
#[derive(Error, Debug)]
pub enum CharSubError {
    /// If a file contains a map-from value but no map-to value, this error will be given. Using
    /// a `CharSubMachine` to delete input strings is not supported. I suppose, if it were really
    /// desired, mapping to some no-op non-printing Unicode code point could work.
    #[error("Missing Map-to value in line: {0}")]
    MissingMapToValue(String),
}


////////////////////////////////// Internal functions

#[derive(Debug)]
struct SubstitutionTrie {
    output: Option<String>,
    children: HashMap<char, SubstitutionTrie>,
}

impl SubstitutionTrie {
    fn new() -> SubstitutionTrie {
        SubstitutionTrie {
            output: None, // The root does not map anything
            children: HashMap::new(),
        }
    }

    // Consumes output
    fn add(&mut self, input: &str, output: &str) {
        let mut current_child = self;
        for ch in input.chars() {
            current_child = current_child
                .children
                .entry(ch)
                .or_insert_with(|| SubstitutionTrie::new());
        }
        if current_child.output.is_some() {
            let old_output = current_child.output.as_ref().unwrap();
            println!(
                "Overwriting mapping {}->{} with {}",
                &input, old_output, &output
            );
        }
        current_child.output = Some(output.to_string());
    }
}

// Takes a line of input and, if successfully parsed, returns `Ok(Some(input,output)))` if there was a
// mapping and `Ok(None)` for a blank line or comment. A line with invalid escape codes will return
// `Err()`
fn parse_charsub_config_line(line: &str) -> anyhow::Result<Option<(String, String)>> {
    let first_char = line.chars().next().unwrap_or(' ');
    if first_char.is_whitespace() {
        return Ok(None);
    }

    let mut words = line.split_whitespace();

    let map_from = words.next().unwrap();

    let map_to = words
        .next()
        .ok_or(CharSubError::MissingMapToValue(line.to_string()))?;

    Ok(Some((map_from.to_string(), map_to.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io;
    use std::path::PathBuf;

    #[test]
    fn blank_lines_and_comments_ignored() -> anyhow::Result<()> {
        assert_eq!(None, parse_charsub_config_line("")?);
        assert_eq!(None, parse_charsub_config_line(" ")?);
        assert_eq!(None, parse_charsub_config_line("\t")?);
        assert_eq!(None, parse_charsub_config_line("  Comment")?);
        assert_eq!(None, parse_charsub_config_line("\u{a0}Comment")?);
        assert_eq!(Some(("a".to_string(), "b".to_string())), parse_charsub_config_line("a b comment")?);

        Ok(())
    }

    #[test]
    fn basic_maps_work_correctly() -> anyhow::Result<()> {
        assert_eq!(
            Some(("``".to_string(), "”".to_string())),
            parse_charsub_config_line("``   \u{201D}")?
        );
        assert_eq!(
            Some(("---".to_string(), "—".to_string())),
            parse_charsub_config_line("---\t\u{2014}")?
        );
        Ok(())
    }

    #[test]
    fn missing_map_to_value_gives_error() {
        assert_eq!(true, parse_charsub_config_line("wrong  ").is_err());
        assert_eq!(true, parse_charsub_config_line("alsoWrong").is_err());
    }

    #[test]
    fn char_sub_machine_returns_original_string_when_no_substitution() {
        let mut char_sub_machine = CharSubMachine::new();
        assert_eq!("original", char_sub_machine.process("original"));
        char_sub_machine.add_substitution("'", "’");
        assert_eq!("original", char_sub_machine.process("original"));
    }

    #[test]
    fn char_sub_machine_handles_substitutions_mid_string() {
        let mut char_sub_machine = CharSubMachine::new();
        char_sub_machine.add_substitution("'", "’");
        char_sub_machine.add_substitution("''", "”");
        assert_eq!("it’s", char_sub_machine.process("it's"));
    }

    #[test]
    fn char_sub_machine_handles_substitutions_at_end() {
        let mut char_sub_machine = CharSubMachine::new();
        char_sub_machine.add_substitution("'", "’");
        char_sub_machine.add_substitution("''", "”");
        assert_eq!("when”", char_sub_machine.process("when''"));
        assert_eq!("", char_sub_machine.process("'"));
        assert_eq!(Some("'".to_string()), char_sub_machine.unprocessed);
        assert_eq!("”", char_sub_machine.process("'"));
    }
    #[test]
    fn char_sub_machine_can_handle_more_than_one_substitution() {
        let mut char_sub_machine = CharSubMachine::new();
        char_sub_machine.add_substitution("'", "’");
        char_sub_machine.add_substitution("''", "”");
        char_sub_machine.add_substitution("`", "‘");
        char_sub_machine.add_substitution("``", "“");
        assert_eq!("“it’s”", char_sub_machine.process("``it's''"));
    }

    #[test]
    fn char_sub_machine_handles_stopping_mid_substitution_ok() {
        let mut char_sub_machine = CharSubMachine::new();
        char_sub_machine.add_substitution("'", "’");
        char_sub_machine.add_substitution("''", "”");
        char_sub_machine.add_substitution("`", "‘");
        char_sub_machine.add_substitution("``", "“");
        assert_eq!("if", char_sub_machine.process("if'"));
        assert_eq!("’", char_sub_machine.flush());
        assert_eq!("", char_sub_machine.flush());
    }

    #[test]
    fn char_sub_machine_handles_substitutions_where_initial_or_middle_sequences_dont_terminate() {
        let mut char_sub_machine = CharSubMachine::new();
        char_sub_machine.add_substitution("ABC", "$$");
        char_sub_machine.add_substitution("DEF", "!!");
        assert_eq!("AB!!", char_sub_machine.process("ABDEF"));
        // What about the end?
        assert_eq!("$$", char_sub_machine.process("ABCDE"));
        assert_eq!("DE", char_sub_machine.flush());
        assert_eq!("", char_sub_machine.flush());

        char_sub_machine = CharSubMachine::new();
        char_sub_machine.add_substitution("12", "@");
        char_sub_machine.add_substitution("123", "#");
        assert_eq!("1 @ #", char_sub_machine.process("1 12 123"));
    }

    #[test]
    fn flush_will_correctly_handle_a_dead_end_substitution() {
        let mut char_sub_machine = CharSubMachine::new();
        char_sub_machine.add_substitution("A", "a");
        char_sub_machine.add_substitution("ABC", "b");
        assert_eq!("", char_sub_machine.process("AB"));
        assert_eq!("aB".to_string(), char_sub_machine.flush());
    }

    #[test]
    fn can_parse_valid_external_file() -> anyhow::Result<()> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources");
        path.push("tex.charsub");
        let file = File::open(path).unwrap();
        let mut char_sub_machine = CharSubMachine::from_buf_reader(&mut io::BufReader::new(file))?;
        assert_eq!("“¿what?”—he’s thinking", char_sub_machine.process("``?`what?''---he's thinking"));
        Ok(())
    }

    // Temporary test - manually verified
    // #[test]
    // fn can_parse_valid_external_file() -> anyhow::Result<()> {
    //     let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    //     path.push("resources");
    //     path.push("tex.charsub");
    //     let file = File::open(path).unwrap();
    //     let items: Vec<(String, String)> = io::BufReader::new(file)
    //         .lines()
    //         .map(|l| {
    //             let l = l.unwrap();
    //             let rv = parse_charsub_config_line(l.as_str()).unwrap();
    //             match rv {
    //                 None => None,
    //                 Some((from, to)) => Some((from.to_string(), to.to_string())),
    //             }
    //         })
    //         .filter_map(|x| x)
    //         .collect();
    //
    //     println!("{:?}", items);
    //
    //     Ok(())
    // }

    // Temporary test - manually verified
    #[test]
    fn add_some_mappings_to_substitution_trie() {
        let mut trie = SubstitutionTrie::new();
        trie.add("abc", "def");
        trie.add("ab", "asd");
        trie.add("abc", "xyz");

        println!("{:?}", trie);
    }
}
