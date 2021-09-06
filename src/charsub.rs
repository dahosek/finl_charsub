//! ## The charsub config file
//!
//! The config file data is given as a series of mappings with an input, followed by white space and an output, e.g.,
//! ```text
//! '     \u{2019}
//! ```
//! indicates that a single straight quote should be mapped to a typographic apostrophe.
//!
//! Blank lines are ignored. Any line beginning with white space will be treated as a comment and ignored.
//!

use anyhow;
use thiserror::Error;
use std::path::PathBuf;
use std::collections::HashMap;
use std::borrow::Cow;

#[derive(Error, Debug)]
pub enum CharSubError {
    #[error("Missing Map-to value in line: {0}")]
    MissingMapToValue(String)
}

pub struct CharSubMachine {
    tree: SubstitutionTree,
    unprocessed: Option<String>
}

impl CharSubMachine {
    fn new() -> CharSubMachine {
        CharSubMachine {
            tree: SubstitutionTree::new(),
            unprocessed: None
        }
    }

    fn add_substitution(&mut self, input: &str, output: &str) {
        self.tree.add(input, output);
    }

    // Unfinished code
    fn process<'a>(&mut self, input: &'a str) -> Cow<'a, str> {
        let mut curr_node = &self.tree;
        let mut built_value :Option<String> = None;
        let mut in_substitution = false;
        let mut substitution_start = 0;
        for (loc, ch) in input.char_indices() {
            if in_substitution && !curr_node.children.contains_key(&ch) {
                if in_substitution {
                    match &curr_node.output {
                        None => {
                            let (substitution, remainder) = self._flush_substitution(input.get(substitution_start..loc).unwrap());
                            &built_value.as_mut().unwrap().push_str(substitution.as_str());
                            if let Some(remainder) = remainder {
                                &built_value.as_mut().unwrap().push_str(remainder.as_str());
                            }
                        }
                        Some(substitution) => {
                            &built_value.as_mut().unwrap().push_str(substitution.as_str());
                        }
                    }
                    in_substitution = false;
                    curr_node = &self.tree;
                }
            }
            if curr_node.children.contains_key(&ch) {
                if built_value.is_none() {
                    built_value = Some(String::with_capacity(input.len()));
                    &built_value.as_mut().unwrap().push_str(input.get(..loc).unwrap());
                }
                if !in_substitution {
                    substitution_start = loc;
                }
                in_substitution = true;
                curr_node = curr_node.children.get(&ch).unwrap();
            }
            else {
                if let Some(output)= &mut built_value {
                    output.push(ch);
                }
            }
        }
        if in_substitution {
            if curr_node.children.is_empty() {
                &built_value.as_mut().unwrap().push_str(&curr_node.output.as_ref().unwrap().as_str());
            }
            else {
                self.unprocessed = input.get(substitution_start..).map(|s| s.to_string());
            }
        }
        match built_value {
            None => Cow::Borrowed(input),
            Some(_) => Cow::Owned(built_value.unwrap())
        }
    }

    fn flush(&mut self) -> String {
        match &self.unprocessed {
            None => String::new(),
            Some(_) => {
                let unprocessed = self.unprocessed.take().unwrap();
                let (rv_part1, rv_part2) = self._flush_substitution(unprocessed.as_str());
                let mut rv_part1= rv_part1.clone();
                if rv_part2.is_some() {
                    rv_part1.push_str(rv_part2.unwrap().as_str());
                }
                rv_part1
            }
        }
    }

    fn _flush_substitution(&mut self, input: &str) -> (String, Option<String>) {
        let mut curr_node = &self.tree;
        let mut end_of_mapping = 0;
        for (loc, ch) in input.char_indices() {
            // We have already looked at this sequence, so we know that every character in input
            // is mapped in the tree
            curr_node = curr_node.children.get(&ch).unwrap();
            if curr_node.output.is_some() {
                end_of_mapping = loc;
                println!("{}", end_of_mapping);
            }
        }
        match &curr_node.output {
            None => if end_of_mapping == 0 {
                (input.to_string(), None)
            }
            else {
                (self._flush_substitution(input.get(..end_of_mapping).unwrap()).0, input.get(end_of_mapping..).map(|s| s.to_string()))
            },
            Some(output) => (output.clone(), None)
        }
    }
}

////////////////////////////////// Internal functions

#[derive(Debug)]
struct SubstitutionTree {
    output: Option<String>,
    children: HashMap<char, SubstitutionTree>
}

impl SubstitutionTree {
    fn new() -> SubstitutionTree {
        SubstitutionTree {
            output: None, // The root does not map anything
            children: HashMap::new()
        }
    }

    // Consumes output
    fn add(&mut self, input: &str, output: &str) {
        let mut current_child = self;
        for ch in input.chars() {
            current_child = current_child.children.entry(ch).or_insert_with(|| SubstitutionTree::new());
        }
        if current_child.output.is_some() {
            let old_output = current_child.output.as_ref().unwrap();
            println!("Overwriting mapping {}->{} with {}", &input,
                     old_output,
                     &output);
        }
        current_child.output = Some(output.to_string());
    }
}

// Takes a line of input and, if successfully parsed, returns Ok(Some(input,output))) if there was a
// mapping and Ok(None) for a blank line or comment. A line with invalid escape codes will return
// Err()
fn parse_charsub_config_line<'a>(line : &'a str) -> anyhow::Result<Option<(&'a str, &'a str)>> {
    let first_char = line.chars().next().unwrap_or(' ');
    if first_char.is_whitespace() {
        return Ok(None)
    }


    let mut words = line.split_whitespace();

    let map_from = words.next().unwrap();

    let map_to = words.next().ok_or(
        CharSubError::MissingMapToValue(line.to_string())
    )?;

    Ok(Some((map_from, map_to)))
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io;
    use std::io::BufRead;

    #[test]
    fn blank_lines_and_comments_ignored() -> anyhow::Result<()> {
        assert_eq!(None, parse_charsub_config_line("")?);
        assert_eq!(None, parse_charsub_config_line(" ")?);
        assert_eq!(None, parse_charsub_config_line("\t")?);
        assert_eq!(None, parse_charsub_config_line("  Comment")?);
        assert_eq!(None, parse_charsub_config_line("\u{a0}Comment")?);
        assert_eq!(Some(("a","b")), parse_charsub_config_line("a b comment")?);

        Ok(())
    }

    #[test]
    fn basic_maps_work_correctly() -> anyhow::Result<()> {
        assert_eq!(Some(("``", "”")),  parse_charsub_config_line("``   \u{201D}")?);
        assert_eq!(Some(("---", "—")), parse_charsub_config_line("---\t\u{2014}")?);
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

    // Temporary test - manually verified
    #[test]
    fn can_parse_valid_external_file() -> anyhow::Result<()> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources");
        path.push("tex.charsub");
        let file = File::open(path).unwrap();
        let items : Vec<(String, String)> = io::BufReader::new(file).lines()
            .map(|l| {
                let l = l.unwrap();
                let rv = parse_charsub_config_line(l.as_str()).unwrap();
                match rv {
                    None => None,
                    Some((from, to)) => Some((from.to_string(), to.to_string()))
                }
            })
            .filter_map(|x| x)
            .collect();

        println!("{:?}", items);

        Ok(())
    }

    // Temporary test - manually verified
    #[test]
    fn add_some_mappings_to_substitution_tree() {
        let mut tree = SubstitutionTree::new();
        tree.add("abc", "def");
        tree.add("ab", "asd");
        tree.add("abc", "xyz");

        println!("{:?}", tree);
    }
}