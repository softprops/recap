use recap_derive::Recap;

fn default_str() -> String {
    "Some default".into()
}

#[derive(Debug, Eq, PartialEq, Recap)]
#[recap(
    handle_deserialize,
    regex = r"((?P<FirstAttribute>\w+):)?(?P<second_rename>\d+):(?P<ThirdAttribute>\w+)"
)]
#[serde(rename_all = "PascalCase")]
struct Test {
    #[serde(default = "default_str")]
    first_attribute: String,
    #[serde(rename = "second_rename")]
    second_attribute: u32,
    third_attribute: String,
}

#[test]
fn custom_deserialize_preserves_serde_attributes() {
    let test = "42:non_default".parse::<Test>().unwrap();
    assert_eq!(
        test,
        Test {
            first_attribute: "Some default".into(),
            second_attribute: 42,
            third_attribute: "non_default".into(),
        }
    );
}
