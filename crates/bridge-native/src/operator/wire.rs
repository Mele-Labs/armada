//! The page's routes, each a request off the main thread on the port the Fleet connection read, or
//! answered by the walk's mock at once. `crates/ipc/operations/operator_page.toml` and its siblings.

use gpui::{App, AppContext, Entity, Task};
use ipc::operator_board::OperatorDispatched;
use ipc::operator_page::{OperatorAnswerPress, OperatorAnswered, OperatorInFront, OperatorLater, OperatorPage};
use ipc::operator_talk::{OperatorSaid, TalkToOperator};
use ipc::{OperatorActs, OperatorActsList, SetOperatorActs};
use ipc::{DispatchJob, JobDispatched, ParkedIdeas, RemoveParkedIdea};
use serde::de::DeserializeOwned;

use super::scenes::MockFleet;
use crate::fleet::FleetConnection;
use crate::refusal::Refusal;

/// Where the page's answers come from.
pub enum Wire {
    Live(Entity<FleetConnection>),
    /// The walk's and the tests' Fleet, which answers from what it holds.
    Mock(Box<MockFleet>),
}

enum Ask {
    Get(&'static str),
    Post(&'static str, String),
    Bare(&'static str),
}

/// Why a press did not come back answered: Fleet said no, or its reply never arrived.
#[derive(Clone, Debug, PartialEq)]
pub enum Failed {
    Refused(String),
    Lost(String),
}

impl From<Failed> for String {
    fn from(failed: Failed) -> String {
        match failed {
            Failed::Refused(why) | Failed::Lost(why) => why,
        }
    }
}

fn request<T: DeserializeOwned + Send + 'static>(port: u16, ask: Ask, what: &'static str, headline: Option<&'static str>, cx: &App) -> Task<Result<T, String>> {
    let task = told(port, ask, what, headline, cx);
    cx.background_spawn(async move { task.await.map_err(String::from) })
}

/// `ask` off the main thread. Where `headline` names a press, a refusal also raises the failure toast; the page
/// still says why where it said it before.
fn told<T: DeserializeOwned + Send + 'static>(port: u16, ask: Ask, what: &'static str, headline: Option<&'static str>, cx: &App) -> Task<Result<T, Failed>> {
    let answer: Task<Result<T, (Failed, Option<Refusal>)>> = cx.background_spawn(async move {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| (Failed::Lost(why.to_string()), None))?;
        let (method, route, size) = match &ask {
            Ask::Get(path) => ("GET", *path, None),
            Ask::Post(path, body) => ("POST", *path, Some(body.len())),
            Ask::Bare(path) => ("POST", *path, None),
        };
        let sent = match ask {
            Ask::Get(path) => runtime.block_on(crate::transport::get(port, path)),
            Ask::Post(path, body) => runtime.block_on(crate::transport::post(port, path, body)),
            Ask::Bare(path) => runtime.block_on(crate::transport::post_bare(port, path)),
        };
        let (status, bytes) = sent.map_err(|why| (Failed::Lost(why), None))?;
        let bytes = crate::refusal::answered(method, route, size, status, bytes).map_err(|refusal| (Failed::Refused(refusal.to_string()), Some(refusal)))?;
        // A 2xx that does not read is Fleet's answer lost on the way: it may have done the press.
        ipc::decode::<T>(what, &bytes).map_err(|why| (Failed::Lost(why.to_string()), None))
    });
    cx.spawn(async move |cx| {
        answer.await.map_err(|(failed, refusal)| {
            if let (Some(headline), Some(refusal)) = (headline, refusal) {
                cx.update(|cx| crate::toasts::tell_refused(headline, refusal, cx));
            }
            failed
        })
    })
}

fn body(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

impl Wire {
    pub(super) fn port(&self, cx: &App) -> Option<u16> {
        match self {
            Wire::Live(fleet) => fleet.read(cx).held().port(),
            Wire::Mock(_) => None,
        }
    }

    pub fn is_live(&self) -> bool {
        matches!(self, Wire::Live(_))
    }

    fn unreachable<T: Send + 'static>() -> Task<Result<T, String>> {
        Task::ready(Err("Fleet is not connected".into()))
    }

    /// `GET /operator/page`.
    pub fn page(&mut self, cx: &App) -> Task<Result<OperatorPage, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(Ok(mock.page.clone()));
        }
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Get("/operator/page"), "OperatorPage", None, cx))
    }

    /// `POST /operator/page/answer`.
    pub fn answer(&mut self, press: OperatorAnswerPress, cx: &App) -> Task<Result<OperatorAnswered, Failed>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.answer(&press));
        }
        let said = body(&press);
        match self.port(cx) {
            Some(port) => told(port, Ask::Post("/operator/page/answer", said), "OperatorAnswered", Some("Your answer was not taken"), cx),
            None => Task::ready(Err(Failed::Lost("Fleet is not connected".into()))),
        }
    }

    /// `POST /operator/page/later`: last until `until`, RFC 3339, or for the rest of the local day.
    pub fn later(&mut self, id: &str, until: Option<String>, cx: &App) -> Task<Result<OperatorPage, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.later(id, until));
        }
        let said = body(&OperatorLater { id: id.to_string(), until: until.map(ipc::Instant::carried) });
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Post("/operator/page/later", said), "OperatorPage", Some("The card was not put off until later"), cx))
    }

    /// `POST /operator/front`, which takes no body.
    pub fn in_front(&mut self, cx: &App) -> Task<Result<OperatorInFront, String>> {
        if let Wire::Mock(mock) = self {
            mock.fronted += 1;
            return Task::ready(Ok(OperatorInFront { rewriting: false }));
        }
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Bare("/operator/front"), "OperatorInFront", None, cx))
    }

    /// `POST /settings/save` with `features.operator`, the switch Electron's Permissions tab saves.
    pub fn switch(&mut self, on: bool, cx: &App) -> Task<Result<(), String>> {
        if let Wire::Mock(mock) = self {
            if let Some(why) = mock.refuse_switch.take() {
                return Task::ready(Err(why));
            }
            mock.switch(on);
            return Task::ready(Ok(()));
        }
        let changes = [("features.operator".to_string(), Some(ipc::SettingValue::Bool(on)))].into_iter().collect();
        let said = body(&ipc::SaveSettings { changes });
        let Some(port) = self.port(cx) else { return Self::unreachable() };
        let headline = if on { "The Operator was not turned on" } else { "The Operator was not turned off" };
        let saved = request::<ipc::SettingsList>(port, Ask::Post("/settings/save", said), "SettingsList", Some(headline), cx);
        cx.background_spawn(async move { saved.await.map(|_| ()) })
    }

    /// `POST /operator/talk`. The mock holds its answer until a scene gives one, so the wait is seen.
    pub fn say(&mut self, talk: TalkToOperator, cx: &App) -> Option<Task<Result<OperatorSaid, String>>> {
        if let Wire::Mock(mock) = self {
            mock.heard_files = talk.attachments.iter().enumerate().map(|(n, one)| ipc::SentFile { id: format!("sent-{}-{n}", mock.heard.len()), name: one.name.clone(), media_type: one.media_type.clone() }).collect();
            mock.heard.push((talk.words, talk.about));
            return None;
        }
        let said = body(&talk);
        Some(self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Post("/operator/talk", said), "OperatorSaid", Some("Your words were not sent to the Operator"), cx)))
    }

    /// `POST /operator/acts`: what the Operator may do unasked in one project, or unset with none. The answer is every
    /// project's value.
    pub fn set_acts(&mut self, project: String, acts: Option<OperatorActs>, cx: &App) -> Task<Result<OperatorActsList, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.set_acts(&project, acts));
        }
        let said = body(&SetOperatorActs { project, acts });
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Post("/operator/acts", said), "OperatorActsList", Some("What the Operator may do was not saved"), cx))
    }

    /// `POST /sessions/dispatch`: the composer's send and a parked idea's start. The mock answers at once, and with
    /// it the `operator.dispatched` events Fleet would publish, which a live Fleet sends on its stream.
    pub fn dispatch(&mut self, job: DispatchJob, cx: &App) -> Task<Result<(JobDispatched, Vec<OperatorDispatched>), String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(Ok(mock.dispatch(job)));
        }
        let said = body(&job);
        let Some(port) = self.port(cx) else { return Self::unreachable() };
        let sent = request::<JobDispatched>(port, Ask::Post("/sessions/dispatch", said), "JobDispatched", Some("The Job was not started"), cx);
        cx.background_spawn(async move { sent.await.map(|answer| (answer, Vec::new())) })
    }

    /// `GET /sessions/parked`: the ideas kept for later.
    pub fn parked(&mut self, cx: &App) -> Task<Result<ParkedIdeas, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(Ok(ParkedIdeas { ideas: mock.parked.clone() }));
        }
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Get("/sessions/parked"), "ParkedIdeas", None, cx))
    }

    /// `POST /sessions/parked/remove`: one idea goes; the answer is the ideas that are left.
    pub fn remove_parked(&mut self, id: String, cx: &App) -> Task<Result<ParkedIdeas, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.remove_parked(&id));
        }
        let said = body(&RemoveParkedIdea { id });
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Post("/sessions/parked/remove", said), "ParkedIdeas", Some("The idea was not removed"), cx))
    }
}
