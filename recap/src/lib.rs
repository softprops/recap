//! Recap deserializes structures from regex [named capture groups](https://www.regular-expressions.info/named.html)
//! extracted from strings.
//!
//! You may find this crate useful for cases where input is provided as a raw string in a loosely structured format.
//! A common use case for this is when dealing with log file data that is not stored in a particular structed format
//! like JSON but rather in a format that can be prepresented with a pattern.
//!
//! Recap is provides what [envy](https://crates.io/crates/envy) provides environment variables for named regex capture groups
//!
//! 💡 For convenience the [regex](https://crates.io/crates/regex) crate's [`Regex`](https://docs.rs/regex/latest/regex/struct.Regex.html)
//! type is re-exported
//!
//! # Examples
//!
//! Below is an example
//!
//! ```rust
//! use recap::{Regex, from_captures};
//! use serde::Deserialize;
//! use std::error::Error;
//!
//! #[derive(Debug, Deserialize, PartialEq)]
//! struct Example {
//!   foo: String,
//!   bar: String,
//! }
//!
//! fn main() -> Result<(), Box<dyn Error>> {
//!   let pattern = Regex::new(
//!     r#"(?P<foo>\S+)\s(?P<bar>\S+)"#
//!   )?;
//!
//!   let example: Example = from_captures(
//!     &pattern, "hello there"
//!   )?;
//!
//!   assert_eq!(
//!      example,
//!      Example {
//!        foo: "hello".into(),
//!        bar: "there".into()
//!      }
//!   );
//!
//!   Ok(())
//! }
//! ```

pub use regex::Regex;
use serde::de::DeserializeOwned;
use std::convert::identity;

// used in derive crate output
// to derive a static for compiled
// regex
#[cfg(feature = "derive")]
#[doc(hidden)]
pub use lazy_static::lazy_static;

// Re-export for #[derive(Recap)]
#[cfg(feature = "derive")]
#[allow(unused_imports)]
#[macro_use]
extern crate recap_derive;
#[cfg(feature = "derive")]
#[doc(hidden)]
pub use recap_derive::*;

/// A type which encapsulates recap errors
pub type Error = envy::Error;

/// Deserialize a type from named regex capture groups
///
/// See module level documentation for examples
pub fn from_captures<D>(
    re: &Regex,
    input: &str,
) -> Result<D, Error>
where
    D: DeserializeOwned,
{
    let caps = re.captures(input).ok_or_else(|| {
        envy::Error::Custom(format!("No captures resolved in string '{}'", input))
    })?;
    envy::from_iter(
        re.capture_names()
            .map(|maybe_name| {
                maybe_name.and_then(|name| {
                    caps.name(name)
                        .map(|val| (name.to_string(), val.as_str().to_string()))
                })
            })
            .filter_map(identity),
    )
}

#[cfg(test)]
mod tests {
    use super::{from_captures, Regex};
    use serde::Deserialize;

    #[derive(Debug, PartialEq, Deserialize)]
    struct LogEntry {
        foo: String,
        bar: String,
        baz: String,
    }

    #[test]
    fn it_works() {
        assert_eq!(
            from_captures::<LogEntry>(
                &Regex::new(
                    r#"(?x)
                    (?P<foo>\S+)
                    \s+
                    (?P<bar>\S+)
                    \s+
                    (?P<baz>\S+)
                "#
                )
                .unwrap(),
                "one two three"
            ),
            Ok(LogEntry {
                foo: "one".into(),
                bar: "two".into(),
                baz: "three".into()
            })
        )
    }
}
