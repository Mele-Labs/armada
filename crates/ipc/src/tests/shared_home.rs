//! What the shared home's wire must keep true for Bridge's hand-written mirror: where the home
//! will be is left out once it is here, and a choice is three plain strings.

use crate::{decode, encode, HomeNamed, HomePlanned, SetHomeRepository, SharedHomeRead};

#[test]
fn the_planned_home_is_left_out_of_a_read_unless_this_machine_holds_no_copy() {
    let held = SharedHomeRead {
        home: Some(HomeNamed { account: "mele-labs".to_string(), repository: "kit".to_string() }),
        planned: None,
        rows: Vec::new(),
        unreadable: Vec::new(),
    };
    assert_eq!(encode(&held).unwrap(), r#"{"home":{"account":"mele-labs","repository":"kit"},"rows":[],"unreadable":[]}"#);

    let planned = SharedHomeRead {
        home: None,
        planned: Some(HomePlanned { account: "mele-labs".to_string(), repository: "kit".to_string(), exists: true }),
        rows: Vec::new(),
        unreadable: Vec::new(),
    };
    let json = encode(&planned).unwrap();
    assert_eq!(json, r#"{"planned":{"account":"mele-labs","repository":"kit","exists":true},"rows":[],"unreadable":[]}"#);
    assert_eq!(decode::<SharedHomeRead>("a read", json.as_bytes()).unwrap(), planned);
}

#[test]
fn a_choice_of_where_the_home_is_made_names_the_profile_the_owner_and_the_repository() {
    let choice = SetHomeRepository { login: "nick".to_string(), owner: "mele-labs".to_string(), name: "kit".to_string() };
    let json = encode(&choice).unwrap();
    assert_eq!(json, r#"{"login":"nick","owner":"mele-labs","name":"kit"}"#);
    assert_eq!(decode::<SetHomeRepository>("a choice", json.as_bytes()).unwrap(), choice);
    assert!(decode::<SetHomeRepository>("a choice", br#"{"login":"nick","owner":"mele-labs"}"#).is_err(), "a name is part of it");
}
