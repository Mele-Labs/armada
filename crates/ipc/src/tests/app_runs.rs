//! A run, as the stream, the list and the save spell it.

use crate::{
    decode, encode, AppRun, AppRunCheckout, AppRunDraftProcess, AppRunOpensIn, AppRunProcess,
    AppRunLink, AppRunState, Cursor, Delivered, Event, Instant, ManifestId, NamedAppRun, PinAppRunPort,
    SaveAppRun, ServerLink, StreamMessage, UnpinAppRunPort,
};

fn running() -> AppRun {
    AppRun {
        name: "dev".to_string(),
        label: "Dev".to_string(),
        opens_in: AppRunOpensIn::Browser,
        manifest_id: ManifestId::carried("01MF"),
        checkout: AppRunCheckout::Job {
            path: "/repos/armada/.armada/slots/slot-2".to_string(),
            job_id: "01JOB".to_string(),
            branch: Some("feature".to_string()),
            bay: 2,
        },
        processes: vec![
            AppRunProcess {
                name: "web".to_string(),
                state: AppRunState::Ready,
                port: Some(41207),
                pinned_port: Some(41207),
                links: vec![ServerLink { url: "http://localhost:41207".to_string(), name: None }],
                server_id: Some("01SERVER".to_string()),
                ended: None,
            },
            AppRunProcess {
                name: "api".to_string(),
                state: AppRunState::NotStarted,
                port: None,
                pinned_port: None,
                links: Vec::new(),
                server_id: None,
                ended: None,
            },
        ],
        last_used: Some(Instant::carried("2026-10-25T10:00:00.000Z")),
        links: vec![AppRunLink {
            from: "web".to_string(),
            to: "api".to_string(),
            port: 8731,
            found_in: "web/vite.config.js".to_string(),
            brief: "Make `web` read `api`'s port from `ARMADA_PORT_API`, falling back to `8731`, in `web/vite.config.js`.".to_string(),
        }],
    }
}

/// The event carries the run flat beside its own `kind`, so the run's field for it is another
/// name, and the checkout's tag nests under `checkout` without touching it.
#[test]
fn the_event_carries_the_run_whole_beside_its_own_kind() {
    let spelled = encode(&StreamMessage::Event(Delivered {
        cursor: Cursor::at(9),
        event: Event::AppRunChanged(running()),
    }))
    .expect("plain data");
    assert!(spelled.contains(r#""kind":"app_run.changed""#), "{spelled}");
    assert!(spelled.contains(r#""opens_in":"browser""#), "{spelled}");
    assert!(spelled.contains(r#""checkout":{"kind":"job""#), "{spelled}");
    assert!(spelled.contains(r#""state":"not_started""#), "{spelled}");
    assert!(spelled.contains(r#""found_in":"web/vite.config.js""#) && spelled.contains(r#""pinned_port":41207"#), "{spelled}");
    let back: StreamMessage = decode("a stream message", spelled.as_bytes()).expect("reads");
    assert!(matches!(
        back,
        StreamMessage::Event(Delivered { event: Event::AppRunChanged(run), .. }) if run == running()
    ));
    // `about` reads the repository off the top level, as it does for every event that names one.
    assert_eq!(Event::AppRunChanged(running()).about().as_deref(), Some("01MF"));
}

/// An optional field is left out when it is empty, never sent as `null`.
#[test]
fn what_is_empty_is_left_out() {
    let spelled = encode(&running()).expect("plain data");
    let api = spelled.split(r#""name":"api""#).nth(1).expect("the second process");
    // The run's own `last_used` and `links` follow its processes.
    let api = api.split(r#""last_used""#).next().expect("the process alone");
    for absent in ["port", "pinned_port", "links", "server_id", "ended"] {
        assert!(!api.contains(absent), "{absent}: {spelled}");
    }
    assert!(!spelled.contains("null"), "{spelled}");

    let main = AppRun { checkout: AppRunCheckout::Main { path: "/repos/armada".to_string() }, last_used: None, links: Vec::new(), ..running() };
    let spelled = encode(&main).expect("plain data");
    assert!(!spelled.contains(r#""found_in""#), "{spelled}");
    assert!(spelled.contains(r#""checkout":{"kind":"main","path":"/repos/armada"}"#), "{spelled}");
    assert!(!spelled.contains("last_used"), "{spelled}");
}

/// A body names a run, and a Job only where the run is on one; the draft's process hands back what
/// it was read with.
#[test]
fn a_body_names_a_run_and_a_job_only_where_there_is_one() {
    let on_main: NamedAppRun = decode("a run", br#"{"name":"dev"}"#).expect("reads");
    assert_eq!(on_main, NamedAppRun { name: "dev".to_string(), job_id: None });
    assert!(!encode(&on_main).expect("plain data").contains("job_id"));

    let save: SaveAppRun = decode(
        "a run to save",
        br#"{"name":"Web app","processes":[{"name":"web","dir":"web","command":"vite --port ${port}","port_env":"PORT"}],"job_id":"01JOB"}"#,
    )
    .expect("reads");
    assert_eq!(
        save.processes,
        vec![AppRunDraftProcess {
            name: "web".to_string(),
            dir: Some("web".to_string()),
            command: "vite --port ${port}".to_string(),
            port_env: Some("PORT".to_string()),
        }]
    );
    // A process with no folder and no environment variable needs neither written.
    let bare: AppRunDraftProcess = decode("a process", br#"{"name":"a","command":"run"}"#).expect("reads");
    assert_eq!((bare.dir, bare.port_env), (None, None));
}

/// A pin names a process and a number; the run and the Job are named only where there are several.
#[test]
fn a_pin_names_a_process_and_a_number_and_an_unpin_only_the_process() {
    let pin: PinAppRunPort = decode("a pin", br#"{"process":"api","port":8731}"#).expect("reads");
    assert_eq!(pin, PinAppRunPort { process: "api".to_string(), port: 8731, name: None, job_id: None });
    let spelled = encode(&pin).expect("plain data");
    assert!(!spelled.contains("name") && !spelled.contains("job_id") && !spelled.contains("null"), "{spelled}");
    let unpin: UnpinAppRunPort = decode("an unpin", br#"{"process":"api","name":"dev","job_id":"01JOB"}"#).expect("reads");
    assert_eq!(unpin.name.as_deref(), Some("dev"));
    assert!(decode::<PinAppRunPort>("a pin", br#"{"process":"api"}"#).is_err(), "a pin with no number is not one");
}
