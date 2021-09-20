#![warn(missing_docs)]

//! Manage character-sequence substitution
//!
//! ## The charsub config file
//!
//! The config file data is given as a series of mappings with an input, followed by white space and an output, e.g.,
//! ```text
//! '     \u{2019}
//! ```
//! indicates that a single straight quote should be mapped to a typographic apostrophe.
//!
//! Blank lines are ignored. Any line beginning with white space will be treated as a comment and ignored.
//! Similarly, any text following the replacement and white space will be treated as a comment and ignored.
//!

pub mod charsub;
pub mod unescape;
