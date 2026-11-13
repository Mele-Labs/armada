use crate::{decode, encode, CloneRepository, ForgeRepositories, ForgeRepository, Instant};

#[test]
fn a_clone_is_a_url_and_a_parent_and_a_newer_field_does_not_break_it() {
    let asked = CloneRepository {
        url: "/srv/repos/shop.git".to_string(),
        parent: "/projects".to_string(),
    };
    let spelled = encode(&asked).expect("plain data");
    assert_eq!(
        spelled,
        r#"{"url":"/srv/repos/shop.git","parent":"/projects"}"#
    );
    let newer = r#"{"url":"/srv/repos/shop.git","parent":"/projects","depth":1}"#;
    let read: CloneRepository = decode("a clone", newer.as_bytes()).expect("reads");
    assert_eq!(read, asked);
    let no_parent = decode::<CloneRepository>("a clone", br#"{"url":"/srv/repos/shop.git"}"#);
    assert!(no_parent.is_err(), "a parent is required");
}

#[test]
fn a_forge_listing_round_trips_and_an_older_answer_reads_with_nothing_cloned_and_nothing_cut() {
    let listed = ForgeRepositories {
        repositories: vec![ForgeRepository {
            owner: "o".to_string(),
            name: "r".to_string(),
            url: "https://forge.invalid/o/r.git".to_string(),
            private: true,
            pushed_at: Some(Instant::carried("2026-11-12T10:00:00Z")),
            local_path: Some("/projects/r".to_string()),
        }],
        cut: true,
    };
    let spelled = encode(&listed).expect("plain data");
    assert_eq!(decode::<ForgeRepositories>("a listing", spelled.as_bytes()).expect("reads"), listed);

    let older = br#"{"repositories":[{"owner":"o","name":"r","url":"https://forge.invalid/o/r.git","private":false}]}"#;
    let read: ForgeRepositories = decode("a listing", older).expect("reads");
    assert!(!read.cut);
    assert_eq!(read.repositories[0].pushed_at, None);
    assert_eq!(read.repositories[0].local_path, None);
    let spelled = encode(&read).expect("plain data");
    assert!(!spelled.contains("cut") && !spelled.contains("local_path"), "absent while unset: {spelled}");
}
