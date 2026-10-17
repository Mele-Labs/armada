//! The tool `walk` in the run engine: what the agent is told about the place, the two answers it may
//! give, and what holds an answer up. `docs/concepts/pull-review.md`, *Walk*.
//!
//! **The place is Fleet's and never the agent's.** It is validated before the run starts, put in
//! the brief, and written into the kept result by [`walk_read`] from the same value, so a walk can
//! never name a place it was not asked to write against.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ReviewRequest, ReviewResult, WalkHow, WalkPlace, WalkStep};
use serde::Deserialize;

use crate::daemon::Fleet;
use crate::review_runs::{nothing, said};

/// A walk asked for with no place to write it against. A 422.
const NEEDS_A_PLACE: &str = "fleet.review_walk_needs_a_place";

const REQUESTS_AT_MOST: usize = 100;
const SAY_AT_MOST: usize = 240;
const LOOK_AT_MOST: usize = 500;
const PATH_AT_MOST: usize = 500;
const BODY_AT_MOST: usize = 8_000;

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum WalkOut {
    Web { steps: Vec<WalkStep> },
    Api { requests: Vec<ReviewRequest> },
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// The place a walk run is written against: required, and held to the rules a saved place is.
    pub(crate) fn walk_place_required(&self, place: Option<&WalkPlace>) -> Result<WalkPlace, Refusal> {
        let Some(place) = place else {
            return Err(Refusal::Unacceptable(self.pr_refusal(
                NEEDS_A_PLACE,
                "a walk is written against the place the change runs, and none was given",
            )));
        };
        match crate::walk_places::place_problem(place) {
            Some(problem) => Err(self.walk_place_refusal(problem)),
            None => Ok(place.clone()),
        }
    }
}

/// The brief's section on where the change runs. **The command that starts it is left out**: the
/// agent has no use for it, and it is the person's own text.
pub(crate) fn place_section(place: &WalkPlace) -> String {
    let how = match place.how {
        WalkHow::Deployed => "It is already running there.",
        WalkHow::Pull => "Armada pulls this commit down and starts it there when the person walks.",
    };
    let address = match place.address.is_empty() {
        true => String::from("(none: this change has no page)"),
        false => place.address.clone(),
    };
    let api = match place.api.is_empty() {
        true => String::from("(none)"),
        false => place.api.clone(),
    };
    format!(
        "## Where the change runs\n\n{how}\nWeb address: {address}\nAPI base: {api}\n\n\
         Write the walk against this place. You cannot reach it from here, so take every label, role \
         and route from the code in the checkout. Do not put the place in your answer: Armada adds it.\n\n"
    )
}

/// How the answer is shaped, for the brief.
pub(crate) fn walk_contract() -> String {
    "{\"kind\": \"web\", \"steps\": [<step>, ...]} for a change a person sees and presses, or \
     {\"kind\": \"api\", \"requests\": [{\"say\": \"what this request shows\", \"method\": \"GET\" | \"POST\" | \"PUT\" | \
     \"PATCH\" | \"DELETE\", \"path\": \"/path on the API base\", \"body\": \"JSON text, optional\", \"look\": \"what to \
     look for in the response\"}]} for an endpoint with no page. Choose from the change. A web walk plays on the page at \
     the web address; an API walk has at most 100 requests and needs an API base or a web address to send them to. A \
     path starts with a slash and names no host.\n\n\
     A step is {\"say\": \"a line the person reads\", plus exactly one verb}: `press`, `look`, `hover`, `later` or \
     `paste` with a target; `type` with the words, and `into` a target (words ending in a newline end with Enter); `key` \
     with a key such as `Tab` or `Meta+n`, and `on` a target; `select` with words, and `in` a target; `drag` with a \
     target, and `by` {\"x\": pixels, \"y\": pixels}. A target is {\"role\": \"button\", \"name\": \"a string inside its \
     accessible name\", \"exact\": true for the whole name, \"within\": another target to look only inside it}; the role \
     is an ARIA role such as button, tab, textbox or link, or `text` for words the page draws, and `name` and `exact` and \
     `within` are optional (but `text` needs its `name`). At most 200 steps, a caption of at most 240 characters. Take \
     each role and name from the code that draws it. Use `look` to end on what proves the change."
        .to_string()
}

/// The path of a request: on the API's base address and nowhere else.
fn path_problem(path: &str) -> Option<&'static str> {
    let before_query = path.split(['?', '#']).next().unwrap_or("");
    let lower = path.to_lowercase();
    if !path.starts_with('/') || path.starts_with("//") {
        Some("starts with a slash and names no host")
    } else if path.contains("://") || path.contains('\\') || path.chars().any(|c| c.is_whitespace() || c.is_control()) {
        Some("names no scheme or host, and has no space in it")
    } else if before_query.split('/').any(|part| part == "..") || lower.contains("%2e%2e") {
        Some("has no `..` in it")
    } else if path.chars().count() > PATH_AT_MOST {
        Some("is at most 500 characters")
    } else {
        None
    }
}

fn requests_held(requests: &[ReviewRequest]) -> Result<(), String> {
    if requests.is_empty() {
        return Err("it has no requests".to_string());
    }
    if requests.len() > REQUESTS_AT_MOST {
        return Err(format!("it has {} requests, and at most {REQUESTS_AT_MOST} are kept", requests.len()));
    }
    for (at, request) in requests.iter().enumerate() {
        let what = format!("request {}", at + 1);
        nothing(&format!("what {what} says"), &request.say)?;
        nothing(&format!("what {what} looks for"), &request.look)?;
        if request.say.chars().count() > SAY_AT_MOST || request.look.chars().count() > LOOK_AT_MOST {
            return Err(format!("{what} says or looks for more than {SAY_AT_MOST} or {LOOK_AT_MOST} characters"));
        }
        if let Some(problem) = path_problem(&request.path) {
            return Err(format!("the path of {what}, `{}`, must be one that {problem}", request.path));
        }
        if let Some(body) = &request.body {
            if body.chars().count() > BODY_AT_MOST {
                return Err(format!("the body of {what} is longer than {BODY_AT_MOST} characters"));
            }
            ipc::decode::<serde::de::IgnoredAny>("a request body", body.as_bytes())
                .map_err(|why| format!("the body of {what} is not JSON: {}", why.why))?;
        }
    }
    Ok(())
}

/// The answer, held to the place it was written against and kept with that place. A walk of a kind
/// the place cannot play is refused.
pub(crate) fn walk_read(json: &str, place: &WalkPlace) -> Result<ReviewResult, String> {
    match said::<WalkOut>(json)? {
        WalkOut::Web { steps } => {
            if place.address.is_empty() {
                return Err("it is a web walk, and the place has an API base and no web address to play it on".to_string());
            }
            if steps.is_empty() {
                return Err("it has no steps".to_string());
            }
            let steps = crate::sessioning::steps_shown(steps)?;
            Ok(ReviewResult::WebWalk { place: place.clone(), steps })
        }
        WalkOut::Api { requests } => {
            requests_held(&requests)?;
            Ok(ReviewResult::ApiWalk { place: place.clone(), requests })
        }
    }
}
