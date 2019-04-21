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
