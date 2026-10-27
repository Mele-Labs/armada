//! What the shared home's wire must keep true for Bridge's hand-written mirror: where the home
//! will be is left out once it is here, and a choice is three plain strings.

use crate::{decode, encode, HomeItem, HomeKind, HomeKinds, HomeNamed, HomePlanned, SetHomeItem, SetHomeRepository, SharedHomeRead};

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

#[test]
fn an_item_switched_on_its_own_is_its_kind_and_name_beside_the_profile() {
    let item = HomeItem { kind: HomeKind::Plugins, name: "impeccable@impeccable".to_string(), repository: None };
    let switch = SetHomeItem { login: "nick".to_string(), item: item.clone(), on: true };
    let json = encode(&switch).unwrap();
    assert_eq!(json, r#"{"login":"nick","kind":"plugins","name":"impeccable@impeccable","on":true}"#);
    assert_eq!(decode::<SetHomeItem>("an item", json.as_bytes()).unwrap(), switch);

    // A read from before items were switched still decodes, with none.
    let kinds = decode::<HomeKinds>("what syncs", br#"{"login":"nick","kinds":[]}"#).unwrap();
    assert!(kinds.items.is_empty());
}
