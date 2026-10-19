//! The owner's time format's rules: exactly the three switches `hour12`, `seconds` and `date`,
//! each true or false.

use crate::time_format::problems;

#[test]
fn three_switches_are_accepted() {
    for text in [
        r#"{"hour12":false,"seconds":true,"date":false}"#,
        r#"{"date":true,"hour12":true,"seconds":true}"#,
    ] {
        assert_eq!(problems(text), Vec::<String>::new(), "{text}");
    }
}

#[test]
fn anything_else_is_named() {
    for (text, first) in [
        ("{", "not JSON"),
        ("[]", "not an object"),
        (r#"{"hour12":true,"seconds":true}"#, "date is missing"),
        (r#"{"hour12":1,"seconds":true,"date":true}"#, "hour12 is not true or false"),
        (r#"{"hour12":true,"seconds":true,"date":true,"zone":true}"#, "zone is not a switch"),
    ] {
        assert_eq!(problems(text).first().map(String::as_str), Some(first), "{text}");
    }
}
