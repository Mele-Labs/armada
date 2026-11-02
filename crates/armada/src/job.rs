//! `armada job` — everything an agent does with its own Job, from one shell command.
//!
//! **A second client of the agent's door, not a second door.** The list is the door's `tools/list`
//! and a call is its `tools/call`, over the same loopback path `armada mcp` posts to, so which acts
//! a Job may call, which Job is calling and how long an answer may be are all the door's decisions.
//! The CLI adds a name for two things, `status` and `dispatch`, and nothing that Fleet serves.

use std::io::Read;
use std::process::ExitCode;

use ipc::job_cli::{acts, list_message, outcome, Act, Outcome};
use ipc::{ManifestSummary, SessionSummaries, SessionSummary, ToolMissing};

use crate::cli::{Body, JobAct, DISPATCH, JOB};
use crate::mcp::{reached, Reached};

/// The act `dispatch` is, as the door lists it.
const STARTS_A_JOB: &str = "start_another_job";

/// Run one `armada job` command: the act's answer on stdout and exit 0, or one sentence on stderr
/// and a non-zero exit.
///
/// **Blocking, on the runtime's own thread**, as `armada mcp` is: one request in flight, and
/// nothing else running to be starved.
pub fn run(asked: JobAct) -> ExitCode {
    match go(asked) {
        Ok(said) => {
            println!("{said}");
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("armada {JOB}: {why}");
            ExitCode::FAILURE
        }
    }
}

fn go(asked: JobAct) -> Result<String, String> {
    if let JobAct::Help { act } = &asked {
        if act == STATUS {
            return Ok(STATUS_HELP.to_string());
        }
    }
    let reached = reached()?;
    match asked {
        JobAct::Status => status(&reached),
        JobAct::Acts => Ok(listing(&offered(&reached)?)),
        JobAct::Help { act } => {
            let all = offered(&reached)?;
            Ok(help(found(&all, &act)?))
        }
        JobAct::Call { act, fields, body } => {
            let all = offered(&reached)?;
            let body = match body {
                None => None,
                Some(Body::Inline(text)) => Some(text),
                Some(Body::Stdin) => Some(stdin()?),
            };
            let fields = match act == STARTS_A_JOB {
                true => repository_named(&reached, &all, fields)?,
                false => fields,
            };
            called(&reached, found(&all, &act)?, &fields, body.as_deref(), session_id().as_deref())
        }
    }
}

const STATUS: &str = "status";

const STATUS_HELP: &str = "status\n    This Job: its id, its repository, whether this folder has a Manifest, the mod's version, \
whether the door is connected and what the Job came up without.\n\narmada job status";

/// What the door lists this caller.
pub(crate) fn offered(reached: &Reached) -> Result<Vec<Act>, String> {
    acts(&posted(reached, &list_message())?)
}

fn found<'a>(all: &'a [Act], name: &str) -> Result<&'a Act, String> {
    all.iter().find(|act| act.name == name).ok_or_else(|| {
        format!(
            "`{name}` is not an act this Job may call. `armada {JOB}` lists them: {}",
            all.iter().map(|act| act.name.as_str()).collect::<Vec<_>>().join(", ")
        )
    })
}

/// One message to the door and the bytes of its answer.
fn posted(reached: &Reached, message: &str) -> Result<Vec<u8>, String> {
    match reached.fleet.post(&reached.door, message.as_bytes()) {
        Ok(answer) if answer.status == 200 && !answer.body.is_empty() => Ok(answer.body),
        Ok(answer) => Err(format!(
            "Armada answered {} to that call rather than a JSON-RPC message",
            answer.status
        )),
        Err(why) => Err(format!("Armada stopped answering: {why}")),
    }
}

/// One act called, and the sentence Fleet refused it with where it did.
pub(crate) fn called(
    reached: &Reached,
    act: &Act,
    fields: &[(String, String)],
    body: Option<&str>,
    session: Option<&str>,
) -> Result<String, String> {
    let mut answer = outcome(&posted(reached, &act.call_message(fields, body, session)?)?);
    // **A body that does not take `session_id` is sent again without it.** The act's schema does not
    // say which bodies read one, and a Job Fleet hosts is placed by its connection anyway.
    if let (Outcome::Refused(why), Some(_)) = (&answer, session) {
        if why.contains("unknown field `session_id`") {
            answer = outcome(&posted(reached, &act.call_message(fields, body, None)?)?);
        }
    }
    match answer {
        Outcome::Done(said) => Ok(said),
        Outcome::Refused(why) => Err(why),
    }
}

/// The Job's own id: what Fleet sets in a hosted Job's process, else what the agent sets in every
/// command it runs, which is how a Job in a terminal is named.
fn session_id() -> Option<String> {
    [adapters::leasing::Holder::SESSION_ENV, adapters::leasing::Holder::AGENT_SESSION_ENV]
        .into_iter()
        .find_map(|name| std::env::var(name).ok().filter(|id| !id.trim().is_empty()))
}

fn stdin() -> Result<String, String> {
    let mut text = String::new();
    std::io::stdin()
        .read_to_string(&mut text)
        .map_err(|why| format!("stdin could not be read: {why}"))?;
    Ok(text)
}

/// `repo=<name>` read as the `manifest_id` of the repository that name is, which is what the act
/// takes. Absent, the act starts the Job in this one's own repository.
fn repository_named(
    reached: &Reached,
    all: &[Act],
    mut fields: Vec<(String, String)>,
) -> Result<Vec<(String, String)>, String> {
    let Some(at) = fields.iter().position(|(name, _)| name == "repo") else {
        return Ok(fields);
    };
    let (_, wanted) = fields.remove(at);
    let manifests = found(all, "list_manifests")
        .map_err(|_| format!("`repo={wanted}` cannot be resolved: this Job is not offered `list_manifests`"))?;
    let listed = called(reached, manifests, &[], None, None)?;
    let held: Vec<ManifestSummary> = ipc::decode("the repositories Fleet serves", listed.as_bytes())
        .map_err(|why| format!("the repositories Fleet serves could not be read: {why}"))?;
    let id = manifest_for(&wanted, &held)?;
    fields.retain(|(name, _)| name != "manifest_id");
    fields.push(("manifest_id".to_string(), id));
    Ok(fields)
}

/// The Manifest a repository's name is: its directory's name as `list_manifests` gives it, any
/// case, or its id. **A name two repositories share is refused naming their paths**, because
/// starting a Job in the wrong one is the mistake this exists to stop.
pub(crate) fn manifest_for(wanted: &str, held: &[ManifestSummary]) -> Result<String, String> {
    let named: Vec<&ManifestSummary> = held
        .iter()
        .filter(|one| one.repository.eq_ignore_ascii_case(wanted) || one.id.as_str() == wanted)
        .collect();
    match named.as_slice() {
        [one] => Ok(one.id.as_str().to_string()),
        [] => Err(format!(
            "no repository is named `{wanted}`. Fleet serves {}",
            match held.is_empty() {
                true => String::from("none"),
                false => held.iter().map(|one| one.repository.as_str()).collect::<Vec<_>>().join(", "),
            }
        )),
        several => Err(format!(
            "`{wanted}` names {} repositories: {}. Pass the id of the one meant as `manifest_id=`",
            several.len(),
            several
                .iter()
                .map(|one| format!("{} ({})", one.id.as_str(), one.path))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// The name an agent types for an act: `dispatch` for the one that starts a Job.
fn typed_as(act: &Act) -> &str {
    match act.name == STARTS_A_JOB {
        true => DISPATCH,
        false => &act.name,
    }
}

/// Every act, one line each, with the CLI's own first.
pub(crate) fn listing(all: &[Act]) -> String {
    let width = all.iter().map(|act| act.name.len()).max().unwrap_or(0).max(STATUS.len());
    let mut lines = vec![format!(
        "{STATUS:<width$}  this Job: its id, repository, mod, door and what is missing"
    )];
    for act in all {
        let alias = match act.name == STARTS_A_JOB {
            true => format!("(also `{DISPATCH}`) "),
            false => String::new(),
        };
        lines.push(format!("{:<width$}  {alias}{}", act.name, first_sentence(&act.description)));
    }
    lines.push(format!(
        "\n`armada {JOB} <act> --help` says what an act takes. An act's answer is JSON on stdout; a refusal is Fleet's sentence on stderr."
    ));
    lines.join("\n")
}

pub(crate) fn first_sentence(description: &str) -> String {
    let end = description.find(". ").unwrap_or(description.len());
    let sentence = description[..end].trim_end_matches('.');
    match sentence.char_indices().nth(120) {
        Some((cut, _)) => format!("{}…", &sentence[..cut]),
        None => sentence.to_string(),
    }
}

/// What one act does and what it takes.
pub(crate) fn help(act: &Act) -> String {
    let mut out = vec![act.name.clone(), format!("    {}", act.description), String::new()];
    let tail = match act.takes_body {
        true => " [<json> | -]",
        false => "",
    };
    out.push(format!("armada {JOB} {} [name=value ...]{tail}", typed_as(act)));
    for field in &act.fields {
        let needed = match field.required {
            true => " (required)",
            false => "",
        };
        out.push(format!("    {}={needed}  {}", field.name, field.description));
    }
    if act.name == STARTS_A_JOB {
        out.push(String::from(
            "    repo=  A repository's name as `list_manifests` gives it, read as `manifest_id`. Absent, this Job's own repository",
        ));
    }
    if act.takes_body {
        out.push(String::from(
            "The body is a JSON object, as one argument or `-` for stdin. Any other `name=value` is a field of it: `true`, `false`, whole numbers and JSON that opens with `{`, `[` or `\"` are read as such, and anything else is text.",
        ));
    }
    out.join("\n")
}

/// This Job, as far as the door can say.
fn status(reached: &Reached) -> Result<String, String> {
    let id = session_id();
    let mut lines = vec![format!("job         {}", id.as_deref().unwrap_or("not named: no Job id in this shell"))];
    lines.push(match &reached.standing {
        Some(standing) => format!(
            "repository  {} (Manifest {})",
            standing.root().file_name().map_or_else(|| String::from("?"), |name| name.to_string_lossy().into_owned()),
            standing.id()
        ),
        None => String::from("repository  none: this folder has no armada.yml, so only this Job's own acts are served"),
    });
    lines.push(match &reached.standing {
        Some(standing) => format!("manifest    {}/armada.yml", standing.root().display()),
        None => String::from("manifest    none"),
    });
    lines.push(format!("door        connected, port {}", reached.fleet.port()));
    let all = offered(reached)?;
    match row_of(reached, &all, id.as_deref()) {
        Ok(Some(row)) => lines.extend(job_lines(&row)),
        Ok(None) => lines.push(String::from("mod         unknown: Fleet's live list does not hold this Job")),
        Err(why) => lines.push(format!("mod         unknown: {why}")),
    }
    Ok(lines.join("\n"))
}

fn row_of(reached: &Reached, all: &[Act], id: Option<&str>) -> Result<Option<SessionSummary>, String> {
    let Some(id) = id else {
        return Ok(None);
    };
    let sessions = found(all, "list_jobs").map_err(|_| String::from("this Job is not offered `list_jobs`"))?;
    let listed = called(reached, sessions, &[], None, None)?;
    let summaries: SessionSummaries = ipc::decode("the live Jobs", listed.as_bytes())
        .map_err(|why| format!("the live Jobs could not be read: {why}"))?;
    Ok(summaries.sessions.into_iter().find(|row| row.id.as_str() == id))
}

pub(crate) fn job_lines(row: &SessionSummary) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(title) = &row.title {
        lines.push(format!("title       {title}"));
    }
    lines.push(match &row.mod_version {
        Some(version) => format!("mod         version {version}, reported"),
        None => String::from("mod         no mod has reported for this Job"),
    });
    lines.push(match row.tools_missing.is_empty() {
        true => String::from("missing     nothing"),
        false => format!(
            "missing     {}",
            row.tools_missing.iter().map(missing_named).collect::<Vec<_>>().join(", ")
        ),
    });
    lines
}

fn missing_named(piece: &ToolMissing) -> &'static str {
    match piece {
        ToolMissing::Mod => "the armada mod did not load",
        ToolMissing::Door => "Fleet's door did not connect",
        ToolMissing::TaskTools => "the task list tools",
    }
}
