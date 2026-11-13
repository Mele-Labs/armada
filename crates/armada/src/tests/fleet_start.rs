//! `armada fleet start` against a launchd that is a fake: what it asks, and what it prints.

use std::cell::RefCell;
use std::path::Path;

use crate::fleet_start::{start, Launchd, Started};
use crate::tests::TempDir;

/// A launchd that answers `print` as told and keeps every other call it was asked.
struct Fake {
    printed: Option<String>,
    refuses: Option<&'static str>,
    asked: RefCell<Vec<String>>,
}

impl Fake {
    fn printing(printed: Option<&str>) -> Self {
        Fake { printed: printed.map(str::to_string), refuses: None, asked: RefCell::new(Vec::new()) }
    }
}

impl Launchd for Fake {
    fn print(&self, service: &str) -> Option<String> {
        self.asked.borrow_mut().push(format!("print {service}"));
        self.printed.clone()
    }

    fn bootstrap(&self, domain: &str, plist: &Path) -> Result<(), String> {
        self.asked.borrow_mut().push(format!("bootstrap {domain} {}", plist.display()));
        self.refuses.map_or(Ok(()), |why| Err(why.to_string()))
    }

    fn kickstart(&self, service: &str) -> Result<(), String> {
        self.asked.borrow_mut().push(format!("kickstart {service}"));
        self.refuses.map_or(Ok(()), |why| Err(why.to_string()))
    }
}

const STOPPED: &str = "gui/501/com.armada.fleet = {\n\tactive count = 0\n\tstate = not running\n}\n";
const RUNNING: &str = "gui/501/com.armada.fleet = {\n\tactive count = 1\n\tstate = running\n\n\tpid = 4242\n\tdetail = {\n\t\tpid = 7\n\t}\n}\n";

#[test]
fn fleet_start_bootstraps_when_not_loaded_kickstarts_when_stopped_and_leaves_a_running_one() {
    let support = TempDir::new();
    let plist = support.path().join("com.armada.fleet.plist");
    std::fs::write(&plist, "<plist/>").unwrap();

    let gone = Fake::printing(None);
    let started = start(&gone, 501, "com.armada.fleet", &plist).unwrap();
    assert_eq!(started, Started::Loaded { label: "com.armada.fleet".into(), plist: plist.clone() });
    assert_eq!(*gone.asked.borrow(), vec!["print gui/501/com.armada.fleet".to_string(), format!("bootstrap gui/501 {}", plist.display())]);
    assert_eq!(started.to_string(), format!("Fleet was not loaded. Loaded com.armada.fleet from {}.", plist.display()));

    let stopped = Fake::printing(Some(STOPPED));
    let started = start(&stopped, 501, "com.armada.fleet", &plist).unwrap();
    assert_eq!(started, Started::Kickstarted { label: "com.armada.fleet".into() });
    assert_eq!(*stopped.asked.borrow(), vec!["print gui/501/com.armada.fleet".to_string(), "kickstart gui/501/com.armada.fleet".to_string()]);
    assert_eq!(started.to_string(), "Fleet was loaded and not running. Started com.armada.fleet.");

    let running = Fake::printing(Some(RUNNING));
    let started = start(&running, 501, "com.armada.fleet", &plist).unwrap();
    assert_eq!(started, Started::AlreadyRunning { pid: 4242 });
    assert_eq!(*running.asked.borrow(), vec!["print gui/501/com.armada.fleet".to_string()], "a running Fleet is not touched");
    assert_eq!(started.to_string(), "Fleet is already running as pid 4242.");
}

#[test]
fn fleet_start_names_the_missing_plist_and_passes_launchctls_own_error_on() {
    let support = TempDir::new();
    let plist = support.path().join("com.armada.fleet.plist");

    let gone = Fake::printing(None);
    let why = start(&gone, 501, "com.armada.fleet", &plist).unwrap_err();
    assert!(why.contains(&plist.display().to_string()), "{why}");
    assert_eq!(gone.asked.borrow().len(), 1, "nothing was bootstrapped");

    std::fs::write(&plist, "<plist/>").unwrap();
    let refusing = Fake { refuses: Some("Bootstrap failed: 5: Input/output error"), ..Fake::printing(None) };
    assert_eq!(start(&refusing, 501, "com.armada.fleet", &plist), Err("Bootstrap failed: 5: Input/output error".into()));
    let refusing = Fake { refuses: Some("Could not find service"), ..Fake::printing(Some(STOPPED)) };
    assert_eq!(start(&refusing, 501, "com.armada.fleet", &plist), Err("Could not find service".into()));
}
