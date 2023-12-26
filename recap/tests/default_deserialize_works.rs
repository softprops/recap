use recap_derive::Recap;
use serde::Deserialize;

#[derive(Debug, Eq, PartialEq, Deserialize, Recap)]
#[recap(regex = r"(?P<first>\w+):(?P<second>\d+)")]
struct Test {
    first: String,
    second: u32,
}

#[test]
fn default_deserialize_works() {
    let test = "hello:1337".parse::<Test>().unwrap();
    assert_eq!(
        test,
        Test {
            first: "hello".into(),
            second: 1337
        }
    );
}
