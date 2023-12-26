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
struct Inner {
    #[serde(default = "default_str")]
    first_attribute: String,
    #[serde(rename = "second_rename")]
    second_attribute: u32,
    third_attribute: String,
}

#[derive(Debug, Eq, PartialEq, Recap)]
#[recap(
    handle_deserialize,
    regex = r"(?P<first>[^ ]+)( (?P<second>[^ ]+))? (?P<third>[^ ]+)"
)]
struct Outer {
    first: Inner,
    second: Option<Inner>,
    third: Option<Inner>,
}

#[test]
fn custom_deserialize_works_with_other_formats() {
    let raw_json = r#"
        {
            "first": {
                "FirstAttribute": "first_first",
                "second_rename": 123,
                "ThirdAttribute": "first_third"
            },
            "third": {
                "second_rename": 456,
                "ThirdAttribute": "third_third"
            }
        }
    "#;
    let outer: Outer = serde_json::from_str(raw_json).unwrap();
    assert_eq!(
        outer,
        Outer {
            first: Inner {
                first_attribute: "first_first".into(),
                second_attribute: 123,
                third_attribute: "first_third".into(),
            },
            second: None,
            third: Some(Inner {
                first_attribute: "Some default".into(),
                second_attribute: 456,
                third_attribute: "third_third".into(),
            }),
        }
    )
}
