//! The sheet on a live Fleet: its repositories are the ones Fleet serves, narrowed to the project the
//! window is scoped to, and Fleet's answers reach it as data. What the sheet draws stays `face.rs`.

use chrono::{DateTime, Duration, Local, NaiveTime, SecondsFormat, TimeZone, Utc};
use gpui::SharedString;
use ipc::{AutopilotProposal, AutopilotSettings, ManifestId, RepositorySummary, StartAfter, StartBase, StartCondition, WaitReading};

use super::face::{AutopilotField, AutopilotRead, Chip, Cycle, Face, Kind, Mark, Mentions, Offered, Part, Reading, Tuning, Wait};

/// A repository Fleet serves: the name its chip carries and the id a Job there starts with.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Served {
    pub name: SharedString,
    pub id: ManifestId,
}

/// The served repositories that can start a Job: each by the name the title bar's picker gives it, with the
/// id it starts on (its Manifest's, else the one Fleet gives a repository with none). One with neither is left out.
pub(super) fn served(repositories: &[RepositorySummary]) -> Vec<Served> {
    let listed = crate::titlebar::repositories_of(repositories, &[]);
    repositories
        .iter()
        .zip(listed)
        .filter_map(|(one, listed)| {
            let name = listed.name;
            let id = one.manifest.as_ref().map(|manifest| manifest.id.clone()).or_else(|| one.implicit_id.clone())?;
            Some(Served { name, id })
        })
        .collect()
}

/// Chips for `names`, in order: the first nine carry a key, the rest go under More.
pub(super) fn chips(names: &[SharedString]) -> (Vec<Chip>, Vec<Chip>) {
    let chip = |at: usize, name: &SharedString| Chip { name: name.clone(), key: (at < 9).then(|| at as u8 + 1), mark: Mark::Plain };
    let shown = names.iter().take(9).enumerate().map(|(at, name)| chip(at, name)).collect();
    let more = names.iter().enumerate().skip(9).map(|(at, name)| chip(at, name)).collect();
    (shown, more)
}

/// The picker offers only `keep`, where the window is scoped to a project: its member repositories. One
/// member is the repository the sheet is for, as the rail's repository is; several leave the words to choose.
pub(super) fn narrowed(mut face: Face, keep: &[SharedString]) -> Face {
    let names: Vec<SharedString> = face.chips.iter().chain(&face.more).map(|chip| chip.name.clone()).filter(|name| keep.contains(name)).collect();
    let (shown, more) = chips(&names);
    face.chips = shown;
    face.more = more;
    face.reading = Reading::Idle;
    if let [only] = names.as_slice() {
        face = read(face, Some(only));
        face.reading = Reading::Scoped;
    }
    face
}

/// The face on a live Fleet. `keep` is the project's member repositories where the window is scoped to one.
pub(super) fn face_of(served: &[Served], keep: Option<&[SharedString]>, jobs: Vec<Offered>, connected: bool, tuning: Tuning) -> Face {
    let names: Vec<SharedString> = served.iter().map(|one| one.name.clone()).collect();
    let (chips, more) = chips(&names);
    let face = Face {
        connected,
        kinds: vec![Kind::Job, Kind::Autopilot],
        chips,
        more,
        reading: Reading::Idle,
        tuning,
        queue: false,
        mentions: Mentions { jobs, ..Mentions::default() },
        wait: None,
        read: None,
        refused: None,
    };
    match keep {
        Some(keep) => narrowed(face, keep),
        None => face,
    }
}

/// What Fleet read of the words: the repository they name is chosen, or none is.
pub(super) fn read(mut face: Face, named: Option<&SharedString>) -> Face {
    for chip in face.chips.iter_mut().chain(face.more.iter_mut()) {
        chip.mark = if Some(&chip.name) == named { Mark::Chosen } else { Mark::Plain };
    }
    face.reading = if named.is_some() { Reading::Detected } else { Reading::Unresolved };
    face
}

/// The default the sheet's efforts and models start from, until Fleet has said its models.
pub(super) fn tuning(models: Option<&ipc::ModelChoices>) -> Tuning {
    Tuning {
        default_model: models.map(|read| read.session.clone().into()),
        models: models.map(|read| read.models.iter().map(|one| one.clone().into()).collect()).unwrap_or_default(),
        efforts: ["low", "medium", "high", "xhigh", "max"].iter().map(|one| one.to_string().into()).collect(),
    }
}

/// Ask Fleet: a POST where there is a body, else a GET. Blocks, so it runs on the background executor. A refusal
/// is Fleet's own sentence.
pub(super) fn call(port: u16, path: String, body: Option<String>) -> Result<Vec<u8>, String> {
    answered(port, path, body, false)
}

/// A POST to a route that asks a model inside the request: the proposal and the wait read wait as long
/// as Electron's do.
pub(super) fn call_model(port: u16, path: String, body: String) -> Result<Vec<u8>, String> {
    answered(port, path, Some(body), true)
}

fn answered(port: u16, path: String, body: Option<String>, model: bool) -> Result<Vec<u8>, String> {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
    let (status, bytes) = runtime.block_on(async {
        match body {
            Some(body) if model => crate::transport::post_to_model(port, &path, body).await,
            Some(body) => crate::transport::post(port, &path, body).await,
            None => crate::transport::get(port, &path).await,
        }
    })?;
    if (200..300).contains(&status) {
        Ok(bytes)
    } else {
        Err(ipc::decode::<ipc::WireError>("WireError", &bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| error.message))
    }
}

// -- Autopilot -----------------------------------------------------------------------------------

/// The labels the Autopilot body's fields carry, which `settings_of` reads them back by.
pub(super) const AT_ONCE: &str = "At once";
pub(super) const UNTIL: &str = "Until";
pub(super) const SPEND: &str = "Spend, in dollars";
pub(super) const JOBS: &str = "Jobs";
pub(super) const WEEKLY: &str = "Weekly usage, in percent";

/// Fleet's reading of a sentence as the body's instruction and fields, each with the words it was read from
/// (`readingOf`, `cockpit/autopilot.ts`). A cap not set is blank.
pub(super) fn autopilot_read(proposal: &AutopilotProposal) -> AutopilotRead {
    let settings = &proposal.settings;
    let said = |words: &Option<String>| words.as_ref().filter(|words| !words.trim().is_empty()).map(|words| SharedString::from(words.clone()));
    let field = |label, value: String, placeholder, quoted| AutopilotField { label, value: value.into(), placeholder, quoted };
    AutopilotRead {
        instruction: settings.instruction.clone().into(),
        fields: vec![
            field(AT_ONCE, settings.at_once.to_string(), "1", said(&proposal.quoted.at_once)),
            field(
                UNTIL,
                settings.until.as_ref().and_then(|at| DateTime::parse_from_rfc3339(at.as_str()).ok()).map(|at| at.with_timezone(&Local).format("%H:%M").to_string()).unwrap_or_default(),
                "no time set",
                said(&proposal.quoted.until),
            ),
            field(SPEND, settings.spend_cap_micros.map(dollars).unwrap_or_default(), "no cap", said(&proposal.quoted.spend_cap_micros)),
            field(JOBS, settings.session_cap.map(|cap| cap.to_string()).unwrap_or_default(), "no cap", said(&proposal.quoted.session_cap)),
            field(WEEKLY, settings.weekly_cap_percent.map(|cap| cap.to_string()).unwrap_or_default(), "no cap", None),
        ],
    }
}

/// A sum as the field is typed and read back: `30`, `12.50` (`dollars`, `cockpit/autopilot.ts`).
fn dollars(micros: u64) -> String {
    let cents = (micros as f64 / 10_000.).round() as u64;
    if cents % 100 == 0 {
        (cents / 100).to_string()
    } else {
        format!("{:.2}", cents as f64 / 100.)
    }
}

fn whole(text: &str) -> Option<u32> {
    let text = text.trim();
    (!text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())).then(|| text.parse().ok()).flatten()
}

/// The next time the person's clock reads `text` (07:00, 7am, 19:30) after `now` (`nextAt`, `cockpit/autopilot.ts`).
fn next_at(text: &str, now: DateTime<Local>) -> Option<DateTime<Local>> {
    let lower = text.trim().to_lowercase();
    let (core, half) = match (lower.strip_suffix("am"), lower.strip_suffix("pm")) {
        (Some(core), _) => (core.trim_end(), Some(false)),
        (_, Some(core)) => (core.trim_end(), Some(true)),
        _ => (lower.as_str(), None),
    };
    let (hours, minutes) = match core.split_once(':') {
        Some((hours, minutes)) if minutes.len() == 2 => (hours, minutes.parse::<u32>().ok()?),
        Some(_) => return None,
        None => (core, 0),
    };
    if hours.is_empty() || hours.len() > 2 || !hours.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let mut hours: u32 = hours.parse().ok()?;
    if half.is_some() && !(1..=12).contains(&hours) {
        return None;
    }
    match half {
        Some(true) if hours < 12 => hours += 12,
        Some(false) if hours == 12 => hours = 0,
        _ => {}
    }
    let time = NaiveTime::from_hms_opt(hours, minutes, 0)?;
    let at = Local.from_local_datetime(&now.date_naive().and_time(time)).earliest()?;
    Some(if at <= now { at + Duration::days(1) } else { at })
}

/// The settings the person confirmed, from the fields as typed (`settingsFrom`, `cockpit/autopilot.ts`). The
/// boxes are not sent, as Electron's dispatch sends none: Fleet reads that as the backlog alone.
pub(super) fn settings_of(instruction: &str, fields: &[(&str, String)], now: DateTime<Local>) -> Result<AutopilotSettings, String> {
    let field = |label: &str| fields.iter().find(|(name, _)| *name == label).map(|(_, value)| value.trim()).unwrap_or("");
    let Some(at_once) = whole(field(AT_ONCE)).filter(|count| *count >= 1) else {
        return Err("At once is a whole number of Jobs, 1 or more.".into());
    };
    let until = match field(UNTIL) {
        "" => None,
        text => Some(ipc::Instant::carried(next_at(text, now).ok_or("Until is a time of day, like 07:00.")?.with_timezone(&Utc).to_rfc3339_opts(SecondsFormat::Millis, true))),
    };
    let spend_cap_micros = match field(SPEND) {
        "" => None,
        text => {
            let spend: f64 = text.trim_start_matches('$').replace(',', "").parse().ok().filter(|spend: &f64| *spend > 0.).ok_or("Spend is an amount in dollars, like 30.")?;
            Some((spend * 1_000_000.).round() as u64)
        }
    };
    let session_cap = match field(JOBS) {
        "" => None,
        text => Some(whole(text).filter(|cap| *cap >= 1).ok_or("Jobs is a whole number, 1 or more, or blank for no cap.")?),
    };
    // The slider's far end is no limit: the plan stops everything at 100 by itself.
    let weekly_cap_percent = match field(WEEKLY) {
        "" | "100" => None,
        text => Some(whole(text).ok_or("Weekly usage is a whole number of percent.")?),
    };
    Ok(AutopilotSettings {
        instruction: instruction.trim().to_string(),
        at_once,
        until,
        spend_cap_micros,
        session_cap,
        weekly_cap_percent,
        project: None,
        on_autopilot: Default::default(),
        decisions: Default::default(),
    })
}

// -- waits ---------------------------------------------------------------------------------------

/// A condition by the key its part of the sentence cycles it by. A part of the plan is named by its id: the
/// plan of the Job waited on is not held here, so its title is not.
pub(super) fn when_key(condition: &StartCondition) -> String {
    match condition {
        StartCondition::Ends => "ends".into(),
        StartCondition::GroupDone { group } => format!("group:{group}"),
        StartCondition::TaskDone { task } => format!("task:{task}"),
        StartCondition::PrPushed => "pushed".into(),
        StartCondition::PrLands => "lands".into(),
    }
}

fn condition_of(key: &str) -> StartCondition {
    match key {
        "pushed" => StartCondition::PrPushed,
        "lands" => StartCondition::PrLands,
        _ => match key.split_once(':') {
            Some(("group", group)) => StartCondition::GroupDone { group: group.to_string() },
            Some(("task", task)) => StartCondition::TaskDone { task: task.to_string() },
            _ => StartCondition::Ends,
        },
    }
}

fn base_key(base: StartBase) -> &'static str {
    match base {
        StartBase::Main => "main",
        StartBase::Branch => "branch",
    }
}

fn options(pairs: &[(&str, &str)]) -> Vec<(SharedString, SharedString)> {
    pairs.iter().map(|(value, label)| (value.to_string().into(), label.to_string().into())).collect()
}

/// What a line of needs offers: all of it, and each thing where it asks for several (`needsOf`, `waits.ts`).
fn needs_options(needs: &str) -> Vec<(SharedString, SharedString)> {
    let words: Vec<&str> = needs.split_whitespace().collect();
    let mut each: Vec<String> = vec![String::new()];
    for word in words {
        if word.eq_ignore_ascii_case("and") {
            each.push(String::new());
        } else if let Some(last) = each.last_mut() {
            if !last.is_empty() {
                last.push(' ');
            }
            last.push_str(word);
        }
    }
    let each: Vec<String> = each.into_iter().filter(|one| !one.is_empty()).collect();
    std::iter::once(needs.to_string()).chain(if each.len() > 1 { each } else { Vec::new() }).map(|one| (one.clone().into(), one.into())).collect()
}

/// The sentence for a reading of the brief that tags `on`. A part a hand set stands while the reading of that
/// part is what it was when he set it (`hand` is the sentence and the reading it was drawn from); a correction
/// in the words that changes the reading takes the part back (`chosen`, `waits.ts`).
pub(super) fn wait_of(on: Offered, others: Vec<Offered>, read: &WaitReading, hand: Option<(&Wait, &WaitReading)>) -> Wait {
    let when_key = when_key(&read.condition);
    let mut when_options = vec![("ends", "ends")];
    let part = match &read.condition {
        StartCondition::GroupDone { .. } => Some("a group of its plan is done"),
        StartCondition::TaskDone { .. } => Some("a task of its plan is done"),
        _ => None,
    };
    if let Some(label) = part {
        when_options.push((when_key.as_str(), label));
    }
    when_options.extend([("pushed", "PR is pushed"), ("lands", "PR lands")]);
    let cycle = |value: &str, options: Vec<(SharedString, SharedString)>| {
        let label = options.iter().find(|(one, _)| one == value).map(|(_, label)| label.clone()).unwrap_or_else(|| value.to_string().into());
        Cycle { value: value.to_string().into(), label, options }
    };
    let mut wait = Wait {
        on,
        when: cycle(&when_key, options(&when_options)),
        base: cycle(base_key(read.base), options(&[("main", "main"), ("branch", "its PR branch")])),
        needs: (!read.needs.is_empty()).then(|| cycle(&read.needs, needs_options(&read.needs))),
        his: Vec::new(),
        others,
    };
    if let Some((held, was)) = hand {
        if held.his(Part::When) && self::when_key(&was.condition) == when_key {
            wait.when = held.when.clone();
            wait.his.push(Part::When);
        }
        if held.his(Part::Base) && was.base == read.base {
            wait.base = held.base.clone();
            wait.his.push(Part::Base);
        }
        if held.his(Part::Needs) && was.needs == read.needs {
            wait.needs = held.needs.clone();
            wait.his.push(Part::Needs);
        }
    }
    wait
}

/// The reading where the proposer could not be had: the Job ends, from main, asking nothing (`waitReading`).
pub(super) fn defaults() -> WaitReading {
    WaitReading { condition: StartCondition::Ends, base: StartBase::Main, needs: String::new(), plain: false }
}

/// The wire's `after` for the sentence as it stands.
pub(super) fn after_of(wait: &Wait) -> StartAfter {
    StartAfter {
        session: ipc::SessionId::carried(wait.on.id.to_string()),
        condition: condition_of(&wait.when.value),
        base: if wait.base.value == "branch" { StartBase::Branch } else { StartBase::Main },
        needs: wait.needs.as_ref().map(|needs| needs.value.to_string()).unwrap_or_default(),
    }
}
