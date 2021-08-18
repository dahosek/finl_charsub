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

#[derive(Error, Debug)]
pub enum CharSubError {
    #[error("Missing Map-to value in line: {0}")]
    MissingMapToValue(String)
}

pub struct CharSubMachine {
    tree: SubstitutionTree,
    output_buffer: String,
}

impl<'a> CharSubMachine {
    fn new() -> CharSubMachine {
        let tree = SubstitutionTree::new();
        CharSubMachine {
            tree: tree,
            output_buffer: String::with_capacity(80),
        }
    }

    fn add_substitution(&mut self, input: &str, output: &str) {
        self.tree.add(input, output);
    }

    fn flush(&mut self) -> Option<String> {
        if self.output_buffer.is_empty() {
            None
        }
        else {
            let return_string = self.output_buffer.clone();
            self.output_buffer.truncate(0);
            Some(return_string)
        }
    }

    fn process(&mut self, input: &str) -> Option<String> {
        None
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
            if !current_child.children.contains_key(&ch) {
                current_child.children.insert(ch, SubstitutionTree::new());
            }
            current_child = current_child.children.get_mut(&ch).unwrap();
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