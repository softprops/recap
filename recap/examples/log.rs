use recap::Recap;
use serde::Deserialize;
use std::error::Error;

#[derive(Debug, PartialEq, Deserialize, Recap)]
#[recap(regex = r#"(?x)
    (?P<foo>\S+)
    \s+
    (?P<bar>\S+)
    \s+
    (?P<baz>\S+)
  "#)]
struct LogEntry {
    foo: String,
    bar: String,
    baz: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let logs = r#"one two three
  four five six"#;
    for line in logs.lines() {
        let entry: LogEntry = line.parse()?;
        println!("{:#?}", entry);
    }
    Ok(())
}
