//! Recap deserializes structures from regex [named capture groups](https://www.regular-expressions.info/named.html)
//! extracted from strings.
//!
//! You may find this crate useful for cases where input is provided as a raw string in a loosely structured format.
//! A common use case for this is when you're dealing with log file data that was not stored in a particular structed format
//! like JSON but rather in a format that can be represented with a pattern.
//!
//! Recap is provides what [envy](https://crates.io/crates/envy) provides environment variables for named regex capture groups
//!
//!
//! # Examples
//!
//! Below is an example that derives a `FromStr` for your type that will
//! parse into the struct using named capture groups
//!
//! ```rust
//! use recap::Recap;
//! use serde::Deserialize;
//! use std::error::Error;
//!
//! #[derive(Debug, Deserialize, PartialEq, Recap)]
//! #[recap(regex=r#"(?P<foo>\S+)\s(?P<bar>\S+)"#)]
//! struct Example {
//!   foo: String,
//!   bar: String,
//! }
//!
//! fn main() -> Result<(), Box<dyn Error>> {
//!
//!   assert_eq!(
//!      "hello there".parse::<Example>()?,
//!      Example {
//!        foo: "hello".into(),
//!        bar: "there".into()
//!      }
//!   );
//!
//!   Ok(())
//! }
//! ```
//!
//! You can also use Recap with Serde's zero-copy deserialization:
//!
//! ```rust
//! use recap::Recap;
//! use serde::Deserialize;
//! use std::convert::TryInto;
//! use std::error::Error;
//!
//! #[derive(Debug, Deserialize, PartialEq, Recap)]
//! #[recap(regex=r#"(?P<foo>\S+)\s(?P<bar>\S+)"#)]
//! struct Example<'a> {
//!   foo: &'a str,
//!   bar: &'a str,
//! }
//!
//! fn main() -> Result<(), Box<dyn Error>> {
//!   let input = "hello there";
//!   let result: Example = input.try_into()?;
//!   assert_eq!(
//!      result,
//!      Example {
//!        foo: "hello",
//!        bar: "there"
//!      }
//!   );
//!
//!   Ok(())
//! }
//! ```
//!
//! You can also use recap by using the generic function `from_captures` in which
//! case you'll be reponsible for bringing your only Regex reference.
//!
//! 💡 For convenience the [regex](https://crates.io/crates/regex) crate's [`Regex`](https://docs.rs/regex/latest/regex/struct.Regex.html)
//! type is re-exported
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
use serde::de::{
    self,
    value::{BorrowedStrDeserializer, MapDeserializer, SeqDeserializer},
    Deserialize, IntoDeserializer,
};

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
type Result<T> = envy::Result<T>;

struct Vars<'a, Iter>(Iter)
where
    Iter: IntoIterator<Item = (&'a str, &'a str)>;

struct Val<'a>(&'a str, &'a str);

impl<'a: 'de, 'de> IntoDeserializer<'de, Error> for Val<'a> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

struct VarName<'a>(&'a str);

impl<'a: 'de, 'de> IntoDeserializer<'de, Error> for VarName<'a> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl<'a, Iter: Iterator<Item = (&'a str, &'a str)>> Iterator for Vars<'a, Iter> {
    type Item = (VarName<'a>, Val<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(k, v)| (VarName(k), Val(k, v)))
    }
}

macro_rules! forward_parsed_values {
    ($($ty:ident => $method:ident,)*) => {
        $(
            fn $method<V>(self, visitor: V) -> Result<V::Value>
                where V: de::Visitor<'de>
            {
                match self.1.parse::<$ty>() {
                    Ok(val) => val.into_deserializer().$method(visitor),
                    Err(e) => Err(de::Error::custom(format_args!("{} while parsing value '{}' provided by {}", e, self.1, self.0)))
                }
            }
        )*
    }
}

impl<'a: 'de, 'de> de::Deserializer<'de> for Val<'a> {
    type Error = Error;
    fn deserialize_any<V>(
        self,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        BorrowedStrDeserializer::new(self.1).deserialize_any(visitor)
    }

    fn deserialize_seq<V>(
        self,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let values = self.1.split(',').map(|v| Val(self.0, v));
        SeqDeserializer::new(values).deserialize_seq(visitor)
    }

    fn deserialize_option<V>(
        self,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    forward_parsed_values! {
        bool => deserialize_bool,
        u8 => deserialize_u8,
        u16 => deserialize_u16,
        u32 => deserialize_u32,
        u64 => deserialize_u64,
        i8 => deserialize_i8,
        i16 => deserialize_i16,
        i32 => deserialize_i32,
        i64 => deserialize_i64,
        f32 => deserialize_f32,
        f64 => deserialize_f64,
    }

    #[inline]
    fn deserialize_newtype_struct<V>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_enum(self.1.into_deserializer())
    }

    serde::forward_to_deserialize_any! {
        char str string unit
        bytes byte_buf map unit_struct tuple_struct
        identifier tuple ignored_any
        struct
    }
}

impl<'a: 'de, 'de> de::Deserializer<'de> for VarName<'a> {
    type Error = Error;
    fn deserialize_any<V>(
        self,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.0.into_deserializer().deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_newtype_struct<V>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    serde::forward_to_deserialize_any! {
        char str string unit seq option
        bytes byte_buf map unit_struct tuple_struct
        identifier tuple ignored_any enum
        struct bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64
    }
}

/// A deserializer for env vars
struct Deserializer<'a, 'de: 'a, Iter: Iterator<Item = (&'a str, &'a str)>> {
    inner: MapDeserializer<'de, Vars<'a, Iter>, Error>,
}

impl<'a, 'de: 'a, Iter: Iterator<Item = (&'a str, &'a str)>> Deserializer<'a, 'de, Iter> {
    fn new(vars: Iter) -> Self {
        Deserializer {
            inner: MapDeserializer::new(Vars(vars)),
        }
    }
}

impl<'a: 'de, 'de, Iter: Iterator<Item = (&'a str, &'a str)>> de::Deserializer<'de>
    for Deserializer<'a, 'de, Iter>
{
    type Error = Error;
    fn deserialize_any<V>(
        self,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_map<V>(
        self,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_map(self.inner)
    }

    serde::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string unit seq
        bytes byte_buf unit_struct tuple_struct
        identifier tuple ignored_any option newtype_struct enum
        struct
    }
}

/// Deserializes a type based on an iterable of `(&str, &str)`
/// representing keys and values
fn from_iter<'a, Iter, T>(iter: Iter) -> Result<T>
where
    T: de::Deserialize<'a>,
    Iter: IntoIterator<Item = (&'a str, &'a str)>,
{
    T::deserialize(Deserializer::new(iter.into_iter()))
}

/// Deserialize a type from named regex capture groups
///
/// See module level documentation for examples
pub fn from_captures<'a, D>(
    re: &'a Regex,
    input: &'a str,
) -> Result<D>
where
    D: Deserialize<'a>,
{
    let caps = re.captures(input).ok_or_else(|| {
        envy::Error::Custom(format!("No captures resolved in string '{}'", input))
    })?;
    from_iter(
        re.capture_names()
            .map(|maybe_name| {
                maybe_name.and_then(|name| caps.name(name).map(|val| (name, val.as_str())))
            })
            .flatten(),
    )
}

#[cfg(test)]
mod tests {
    use super::{from_captures, Regex};
    use serde::Deserialize;
    use std::error::Error;

    #[derive(Debug, PartialEq, Deserialize)]
    struct LogEntry {
        foo: String,
        bar: String,
        baz: String,
    }

    #[derive(Debug, PartialEq, Deserialize)]
    struct LogEntryOptional {
        foo: String,
        bar: String,
        baz: Option<String>,
    }

    #[derive(Debug, PartialEq, Deserialize)]
    struct LogEntryBorrowed<'a> {
        foo: &'a str,
        bar: &'a str,
        baz: &'a str,
    }

    #[test]
    fn deserializes_matching_captures_optional() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            from_captures::<LogEntryOptional>(
                &Regex::new(
                    r#"(?x)
                    (?P<foo>\S+)
                    \s+
                    (?P<bar>\S+)
                    \s+
                    (?P<baz>\S+)?
                "#
                )?,
                "one two "
            )?,
            LogEntryOptional {
                foo: "one".into(),
                bar: "two".into(),
                baz: None
            }
        );

        Ok(())
    }

    #[test]
    fn deserializes_matching_captures() -> Result<(), Box<dyn Error>> {
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
                )?,
                "one two three"
            )?,
            LogEntry {
                foo: "one".into(),
                bar: "two".into(),
                baz: "three".into()
            }
        );

        Ok(())
    }

    #[test]
    fn deserializes_zero_copy() -> Result<(), Box<dyn Error>> {
        let input = "one two three";
        assert_eq!(
            from_captures::<LogEntryBorrowed>(
                &Regex::new(
                    r#"(?x)
                    (?P<foo>\S+)
                    \s+
                    (?P<bar>\S+)
                    \s+
                    (?P<baz>\S+)
                "#
                )?,
                input
            )?,
            LogEntryBorrowed {
                foo: "one",
                bar: "two",
                baz: "three"
            }
        );

        Ok(())
    }

    #[test]
    fn fails_without_captures() -> Result<(), Box<dyn Error>> {
        let result = from_captures::<LogEntry>(&Regex::new("test")?, "one two three");
        match result {
            Ok(_) => panic!("should have failed"),
            // enum variants on type aliases are experimental
            Err(err) => assert_eq!(
                err.to_string(),
                "No captures resolved in string \'one two three\'"
            ),
        }

        Ok(())
    }

    #[test]
    fn fails_with_unmatched_captures() -> Result<(), Box<dyn Error>> {
        let result = from_captures::<LogEntry>(&Regex::new(".+")?, "one two three");
        match result {
            Ok(_) => panic!("should have failed"),
            // enum variants on type aliases are experimental
            Err(err) => assert_eq!(err.to_string(), "missing value for field foo"),
        }

        Ok(())
    }
}
