# recap [![Build Status](https://travis-ci.org/softprops/recap.svg?branch=master)](https://travis-ci.org/softprops/recap) [![Coverage Status](https://coveralls.io/repos/github/softprops/recap/badge.svg)](https://coveralls.io/github/softprops/recap)

> deserialize named capture groups into typesafe structs

Recap is provides what [envy](https://crates.io/crates/envy) provides environment variables for named regex capture groups

## 📦  install

Add the following to your `Cargo.toml` file.

```toml
[dependencies]
recap = "0.1"
```

## 🤸 usage

A typical recap usage looks like the following. Assuming your rust program looks something like this...

> 💡 These examples use Serde's [derive feature](https://serde.rs/derive.html)

```rust
use recap::Recap;
use serde::Deserialize;
use std::error::Error;

#[derive(Debug, PartialEq, Deserialize, Recap)]
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