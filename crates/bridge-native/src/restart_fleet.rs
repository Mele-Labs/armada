//! Restart Fleet through launchd, as `apps/desktop/src/main/restart-fleet.ts` does. **Only a Fleet the
//! service holds is restarted**: kickstarting the job beside a Fleet started by hand would run two.
//! Blocking; the caller runs it off the main thread.

use std::process::Command;

use crate::titlebar::RestartWhy;

/// The launchd label Armada's service runs under, unless `ARMADA_FLEET_LABEL` names another.
const LABEL: &str = "com.armada.fleet";

/// Restart the Fleet the runtime file names as `pid`, or say why not with what launchd answered.
pub fn restart(pid: Option<u32>) -> Result<(), (RestartWhy, String)> {
    let Some(pid) = pid else {
        return Err((RestartWhy::NotRunning, "the runtime file names no Fleet".into()));
    };
    let label = std::env::var("ARMADA_FLEET_LABEL").unwrap_or_else(|_| LABEL.into());
    let uid = Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|text| text.trim().to_string())
        .unwrap_or_else(|| "0".into());
    let service = format!("gui/{uid}/{label}");
    let held = Command::new("launchctl").args(["print", &service]).output();
    let loaded = held.as_ref().is_ok_and(|out| out.status.success());
    let holds = held.ok().and_then(|out| {
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .find_map(|line| line.trim().strip_prefix("pid = ").and_then(|held| held.trim().parse::<u32>().ok()))
    });
    if !loaded {
        return Err((RestartWhy::NotStartedByArmada, format!("{service} is not loaded")));
    }
    if holds != Some(pid) {
        let held = holds.map_or("none".to_string(), |held| held.to_string());
        return Err((RestartWhy::NotStartedByArmada, format!("{service} holds pid {held}, the runtime file names {pid}")));
    }
    match Command::new("launchctl").args(["kickstart", "-k", &service]).output() {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => {
            let said = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let code = out.status.code().unwrap_or(1);
            Err((RestartWhy::Refused, if said.is_empty() { format!("launchctl exited {code}") } else { said }))
        }
        Err(why) => Err((RestartWhy::Refused, why.to_string())),
    }
}
