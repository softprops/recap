use recap::Recap;

#[derive(Debug, Eq, PartialEq, Recap)]
#[recap(handle_deserialize, regex = r"(?P<foo>\w+):(?P<bar>\d+)")]
struct Inner {
    foo: String,
    bar: u32,
}

#[derive(Debug, Eq, PartialEq, Recap)]
#[recap(handle_deserialize, regex = r"(?P<first>[^ ]+)( (?P<second>[^ ]+))?")]
struct Outer {
    first: Inner,
    second: Option<Inner>,
}

#[test]
fn custom_deserialize_allows_nested_structs() {
    let outer: Outer = "abc:123 def:456".try_into().unwrap();
    assert_eq!(
        outer,
        Outer {
            first: Inner {
                foo: "abc".to_owned(),
                bar: 123
            },
            second: Some(Inner {
                foo: "def".to_owned(),
                bar: 456,
            }),
        }
    );
    let outer: Outer = "ghi:789".try_into().unwrap();
    assert_eq!(
        outer,
        Outer {
            first: Inner {
                foo: "ghi".to_owned(),
                bar: 789
            },
            second: None,
        }
    );
}
