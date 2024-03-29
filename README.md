# recap [![Main](https://github.com/softprops/recap/actions/workflows/main.yml/badge.svg)](https://github.com/softprops/recap/actions/workflows/main.yml) [![Software License](https://img.shields.io/badge/license-MIT-brightgreen.svg)](LICENSE) [![crates.io](https://img.shields.io/crates/v/recap.svg)](https://crates.io/crates/recap) [![Released API docs](https://docs.rs/recap/badge.svg)](http://docs.rs/recap) [![Master API docs](https://img.shields.io/badge/docs-master-green.svg)](https://softprops.github.io/recap)

> deserialize named capture groups into typesafe structs

Recap is provides what [envy](https://crates.io/crates/envy) provides for environment variables, for[ named capture groups](https://www.regular-expressions.info/named.html). Named regex capture groups are like any other regex capture group but have the extra property that they are associated with name. i.e `(?P<name-of-capture-group>some-pattern)`

## 🤔 who is this for

You may find this crate useful for cases where your application needs to extract information from string input provided by a third party that has a loosely structured format.

A common usecase for this is when you are dealing with log file data that was not stored in a particular structured format like JSON, but rather in a format that can be represented with a pattern.

You may also find this useful parsing other loosely formatted data patterns.

This crate would be less appropriate for cases where your input is provided in a more structured format, like JSON.
I recommend using a crate like [`serde-json`](https://crates.io/crates/serde_json) for those cases instead.

## 📦 install

Add the following to your `Cargo.toml` file.

```toml
[dependencies]
recap = "0.1"
```

## 🤸 usage

A typical recap usage looks like the following. Assuming your Rust program looks something like this...

> 💡 These examples use Serde's [derive feature](https://serde.rs/derive.html)

```rust
use recap::Recap;
use serde::Deserialize;
use std::error::Error;

#[derive(Debug, Deserialize, Recap)]
#[recap(regex = r#"(?x)
    (?P<foo>\d+)
    \s+
    (?P<bar>true|false)
    \s+
    (?P<baz>\S+)
  "#)]
struct LogEntry {
    foo: usize,
    bar: bool,
    baz: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let logs = r#"1 true hello
  2 false world"#;

    for line in logs.lines() {
        let entry: LogEntry = line.parse()?;
        println!("{:#?}", entry);
    }

    Ok(())
}

```

> 👭 Consider this crate a cousin of [envy](https://github.com/softprops/envy), a crate for deserializing environment variables into typesafe structs.

Doug Tangren (softprops) 2019
