//! How the native Bridge finds Fleet: the runtime file, the pid check and the protocol check.
//!
//! Mirrors `apps/desktop/src/main/runtime-file.ts`, which mirrors `crates/fleet/src/runtime.rs`.
//! Unknown fields are ignored; a file that will not parse is refused rather than read as "not
//! running", because Fleet writes it by rename and so a torn file was not written by Fleet.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{Local, NaiveDateTime, TimeZone};
use serde::Deserialize;

use ipc::ProtocolId;

const FILE_NAME: &str = "fleet.json";

/// The largest pid the platform can express. Above it names no process.
const PID_CEILING: u32 = 2_147_483_647;

/// Where Fleet publishes itself: `~/Library/Application Support/Armada/fleet.json`.
pub fn path(home: Option<&str>) -> Option<PathBuf> {
    let home = home.filter(|home| !home.is_empty())?;
    Some(Path::new(home).join("Library").join("Application Support").join("Armada").join(FILE_NAME))
}

/// The four fields Fleet writes. Loopback is a constant on both sides, so the host is not one.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct RuntimeFile {
    /// A file written before IDs has none, which reads as an ID nothing equals.
    #[serde(default)]
    pub protocol_id: ProtocolId,
    pub pid: u32,
    pub port: u16,
    /// As `ps -o lstart=` printed it when Fleet wrote the file.
    pub started_at: String,
    /// The kept build folder Fleet runs from, such as `2026-11-13T004852Z-126b4bb`. None where it
    /// runs from elsewhere, or wrote the file before it said.
    #[serde(default)]
    pub build: Option<String>,
}

/// Who holds a pid, as `ps` reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Holder {
    Vacant,
    Held { started_at: String },
    /// The probe itself failed, which is not the same as nothing being there.
    Failed(String),
}

/// What is at the runtime file's path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Presence {
    Absent,
    /// A file whose pid is dead, or is held by a process that started at another time.
    Stale(String),
    Refused(String),
    Running(RuntimeFile),
}

/// Ask `ps` who holds `pid`. The same spelling Fleet and Electron use, so all three agree.
pub fn holder_of(pid: u32) -> Holder {
    if pid == 0 || pid > PID_CEILING {
        return Holder::Vacant;
    }
    let run = match Command::new("ps").args(["-o", "lstart=", "-p", &pid.to_string()]).output() {
        Ok(run) => run,
        Err(why) => return Holder::Failed(why.to_string()),
    };
    // Told apart by stderr: an absent pid and a refused argument both exit non-zero.
    let complaint = String::from_utf8_lossy(&run.stderr).trim().to_string();
    if !complaint.is_empty() {
        return Holder::Failed(complaint);
    }
    let reading = String::from_utf8_lossy(&run.stdout).trim().to_string();
    if reading.is_empty() {
        Holder::Vacant
    } else {
        Holder::Held { started_at: reading }
    }
}

/// Decide what the file's bytes, or the failure to read them, describe. The probe is part of
/// reading, so no pid reaches a connection attempt unchecked.
pub fn classify(read: io::Result<Vec<u8>>, probe: impl Fn(u32) -> Holder) -> Presence {
    let bytes = match read {
        Ok(bytes) => bytes,
        Err(why) if why.kind() == io::ErrorKind::NotFound => return Presence::Absent,
        Err(why) => return Presence::Refused(format!("the runtime file could not be read: {why}")),
    };
    let file: RuntimeFile = match ipc::decode("runtime file", &bytes) {
        Ok(file) => file,
        Err(why) => return Presence::Refused(why.to_string()),
    };
    match probe(file.pid) {
        Holder::Failed(detail) => Presence::Refused(format!("the pid probe failed: {detail}")),
        Holder::Vacant => Presence::Stale(format!("pid {} is not running", file.pid)),
        Holder::Held { started_at } if started_at != file.started_at => Presence::Stale(format!(
            "pid {} started at {started_at}, not {}",
            file.pid, file.started_at
        )),
        Holder::Held { .. } => Presence::Running(file),
    }
}

/// `Err` carries what Fleet speaks. Checked before connecting, so a protocol this build will not
/// speak is a refusal and not a malformed first message.
pub fn speaks_ours(file: &RuntimeFile) -> Result<(), ProtocolId> {
    if file.protocol_id == ProtocolId::current() {
        Ok(())
    } else {
        Err(file.protocol_id.clone())
    }
}

/// How long ago `started_at` was, in milliseconds, or `None` where the text will not parse
/// (which says nothing about age).
pub fn age_ms(started_at: &str, now: chrono::DateTime<Local>) -> Option<i64> {
    let naive = NaiveDateTime::parse_from_str(started_at, "%a %b %e %H:%M:%S %Y").ok()?;
    let born = Local.from_local_datetime(&naive).single()?;
    Some((now - born).num_milliseconds())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(protocol: &str) -> String {
        format!(
            r#"{{"protocol_id":"{protocol}","pid":14991,"port":40000,"started_at":"Fri Nov  6 22:01:34 2026","extra":1}}"#
        )
    }

    fn held(started_at: &str) -> impl Fn(u32) -> Holder + '_ {
        move |_| Holder::Held { started_at: started_at.to_string() }
    }

    #[test]
    fn the_path_is_under_application_support() {
        assert_eq!(
            path(Some("/Users/user")),
            Some(PathBuf::from("/Users/user/Library/Application Support/Armada/fleet.json"))
        );
        assert_eq!(path(Some("")), None);
        assert_eq!(path(None), None);
    }

    #[test]
    fn a_missing_file_is_absent() {
        let gone = io::Error::from(io::ErrorKind::NotFound);
        assert_eq!(classify(Err(gone), |_| Holder::Vacant), Presence::Absent);
    }

    #[test]
    fn an_unreadable_file_is_refused() {
        let denied = io::Error::from(io::ErrorKind::PermissionDenied);
        assert!(matches!(classify(Err(denied), |_| Holder::Vacant), Presence::Refused(_)));
    }

    #[test]
    fn a_file_that_will_not_parse_is_refused_not_absent() {
        assert!(matches!(classify(Ok(b"{".to_vec()), |_| Holder::Vacant), Presence::Refused(_)));
        let missing_port = br#"{"protocol_id":"x","pid":1,"started_at":"s"}"#.to_vec();
        assert!(matches!(classify(Ok(missing_port), |_| Holder::Vacant), Presence::Refused(_)));
    }

    #[test]
    fn unknown_fields_are_ignored_and_a_missing_protocol_id_reads_as_none() {
        let old = br#"{"pid":1,"port":2,"started_at":"s"}"#.to_vec();
        let Presence::Running(read) = classify(Ok(old), held("s")) else { panic!("not running") };
        assert_eq!(read.protocol_id, ProtocolId::default());
        assert!(speaks_ours(&read).is_err());
    }

    #[test]
    fn the_kept_build_fleet_runs_from_is_read_and_an_older_file_has_none() {
        let named = br#"{"pid":1,"port":2,"started_at":"s","build":"2026-11-13T004852Z-126b4bb"}"#.to_vec();
        let Presence::Running(read) = classify(Ok(named), held("s")) else { panic!("not running") };
        assert_eq!(read.build.as_deref(), Some("2026-11-13T004852Z-126b4bb"));
        let Presence::Running(old) = classify(Ok(file("a").into_bytes()), held("Fri Nov  6 22:01:34 2026")) else { panic!("not running") };
        assert_eq!(old.build, None);
    }

    #[test]
    fn a_dead_pid_is_stale() {
        let read = classify(Ok(file("a").into_bytes()), |_| Holder::Vacant);
        assert!(matches!(read, Presence::Stale(_)));
    }

    #[test]
    fn a_pid_held_by_another_process_is_stale() {
        let read = classify(Ok(file("a").into_bytes()), held("Sat Nov  7 01:00:00 2026"));
        assert!(matches!(read, Presence::Stale(_)));
    }

    #[test]
    fn a_failed_probe_is_refused() {
        let read = classify(Ok(file("a").into_bytes()), |_| Holder::Failed("no ps".into()));
        assert!(matches!(read, Presence::Refused(_)));
    }

    #[test]
    fn the_same_process_is_running() {
        let read = classify(Ok(file("a").into_bytes()), held("Fri Nov  6 22:01:34 2026"));
        let Presence::Running(running) = read else { panic!("not running") };
        assert_eq!((running.pid, running.port), (14991, 40000));
    }

    #[test]
    fn only_this_builds_protocol_id_is_spoken() {
        let ours = ProtocolId::current();
        let same = classify(Ok(file(ours.as_str()).into_bytes()), held("Fri Nov  6 22:01:34 2026"));
        let Presence::Running(same) = same else { panic!("not running") };
        assert!(speaks_ours(&same).is_ok());
        let other = classify(Ok(file("0000000000000000").into_bytes()), held("Fri Nov  6 22:01:34 2026"));
        let Presence::Running(other) = other else { panic!("not running") };
        assert_eq!(speaks_ours(&other), Err(other.protocol_id.clone()));
    }

    #[test]
    fn pids_that_name_no_process_are_vacant() {
        assert_eq!(holder_of(0), Holder::Vacant);
        assert_eq!(holder_of(PID_CEILING + 1), Holder::Vacant);
    }

    #[test]
    fn started_at_parses_as_local_time() {
        let at = Local.with_ymd_and_hms(2026, 11, 6, 22, 1, 34).single().unwrap();
        let later = at + chrono::Duration::seconds(90);
        assert_eq!(age_ms("Fri Nov  6 22:01:34 2026", later), Some(90_000));
        assert_eq!(age_ms("not a date", later), None);
    }
}
