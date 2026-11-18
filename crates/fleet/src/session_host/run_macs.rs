//! The Mac an agent's `run` item is meant for (`WaitingAct.on`).
//!
//! **Only the act says it**: Fleet does not read the Mac out of the item's words. Bridge offers Run only where
//! the act names this Mac or none, so a command meant for the other Mac is copied, not run here.

use ipc::{WaitingAct, WaitingActKind};

/// A Mac Fleet knows, by its id and its name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KnownMac {
    pub id: String,
    pub name: String,
}

/// The act with `on` settled: trimmed, empty taken as none, and a known Mac named by its name whether the agent
/// gave its id or its name. A name Fleet does not know stands while no Mac is paired, since Fleet then has
/// nothing to check it against. Refused with the sentence the agent reads where `on` is on an act that is not a
/// `run`, or names neither this Mac nor a paired one.
pub(crate) fn on_settled(id: &str, act: &mut WaitingAct, this: Option<&KnownMac>, paired: &[KnownMac]) -> Result<(), String> {
    let Some(asked) = act.on.as_deref().map(str::trim).filter(|on| !on.is_empty()).map(str::to_string) else {
        act.on = None;
        return Ok(());
    };
    if act.kind != WaitingActKind::Run {
        return Err(format!(
            "Refused: item \"{id}\" names a Mac on an act that is not run. Only a run act's command is meant for a \
             Mac. Call waiting_for again without act.on on it."
        ));
    }
    let known = this.into_iter().chain(paired);
    let found = known.clone().find(|mac| mac.id == asked || mac.name.eq_ignore_ascii_case(&asked));
    match (found, paired.is_empty()) {
        (Some(mac), _) => act.on = Some(mac.name.clone()),
        (None, true) => act.on = Some(asked),
        (None, false) => {
            let names: Vec<String> = this
                .map(|mac| format!("\"{}\" (this Mac)", mac.name))
                .into_iter()
                .chain(paired.iter().map(|mac| format!("\"{}\"", mac.name)))
                .collect();
            return Err(format!(
                "Refused: item \"{id}\" is for Mac \"{asked}\", which is neither this Mac nor one paired with it. \
                 The Macs Fleet knows are {}. Pass one of those as act.on, or leave it out for this Mac.",
                names.join(" and ")
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mac(id: &str, name: &str) -> KnownMac {
        KnownMac { id: id.into(), name: name.into() }
    }

    fn run(on: Option<&str>) -> WaitingAct {
        WaitingAct { kind: WaitingActKind::Run, target: "curl localhost:4000/v1/peers".into(), on: on.map(str::to_string) }
    }

    #[test]
    fn a_run_item_for_a_paired_mac_is_kept_under_that_macs_name() {
        let this = mac("m1", "Nick's MacBook Pro");
        let paired = [mac("m2", "Nick-Meles-MacBook-Pro---Kiavi")];
        let mut by_name = run(Some(" nick-meles-macbook-pro---kiavi "));
        on_settled("peers", &mut by_name, Some(&this), &paired).unwrap();
        assert_eq!(by_name.on.as_deref(), Some("Nick-Meles-MacBook-Pro---Kiavi"));
        let mut by_id = run(Some("m2"));
        on_settled("peers", &mut by_id, Some(&this), &paired).unwrap();
        assert_eq!(by_id.on.as_deref(), Some("Nick-Meles-MacBook-Pro---Kiavi"));
        let mut here = run(Some("m1"));
        on_settled("peers", &mut here, Some(&this), &paired).unwrap();
        assert_eq!(here.on.as_deref(), Some("Nick's MacBook Pro"));
    }

    #[test]
    fn a_run_item_for_a_mac_fleet_does_not_know_is_refused_naming_the_known_ones() {
        let this = mac("m1", "Nick's MacBook Pro");
        let paired = [mac("m2", "Kiavi")];
        let why = on_settled("peers", &mut run(Some("Studio")), Some(&this), &paired).unwrap_err();
        assert!(why.contains("\"Studio\""), "{why}");
        assert!(why.contains("\"Nick's MacBook Pro\" (this Mac) and \"Kiavi\""), "{why}");
    }

    #[test]
    fn a_name_stands_while_no_mac_is_paired_and_a_blank_one_is_none() {
        let this = mac("m1", "Nick's MacBook Pro");
        let mut other = run(Some("Kiavi"));
        on_settled("peers", &mut other, Some(&this), &[]).unwrap();
        assert_eq!(other.on.as_deref(), Some("Kiavi"));
        let mut blank = run(Some("  "));
        on_settled("peers", &mut blank, Some(&this), &[]).unwrap();
        assert_eq!(blank.on, None);
    }

    #[test]
    fn a_mac_on_an_act_that_is_not_run_is_refused() {
        let mut answer = WaitingAct { kind: WaitingActKind::Answer, target: "call".into(), on: Some("Kiavi".into()) };
        assert!(on_settled("q", &mut answer, None, &[]).is_err());
    }
}
