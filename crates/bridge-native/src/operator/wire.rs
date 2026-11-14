//! The page's routes, each a request off the main thread on the port the Fleet connection read, or
//! answered by the walk's mock at once. `crates/ipc/operations/operator_page.toml` and its siblings.

use gpui::{App, AppContext, Entity, Task};
use ipc::operator_board::OperatorDispatched;
use ipc::operator_page::{OperatorAnswerPress, OperatorAnswered, OperatorInFront, OperatorLater, OperatorPage, OperatorTurnOrder, OperatorUndoRuling};
use ipc::operator_talk::{OperatorOrders, OperatorSaid, OperatorTalk, RemoveOperatorOrder, TalkToOperator};
use ipc::{DispatchJob, JobDispatched, ParkedIdeas, RemoveParkedIdea};
use serde::de::DeserializeOwned;

use super::scenes::MockFleet;
use crate::fleet::FleetConnection;

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

fn request<T: DeserializeOwned + Send + 'static>(port: u16, ask: Ask, what: &'static str, cx: &App) -> Task<Result<T, String>> {
    let task = told(port, ask, what, cx);
    cx.background_spawn(async move { task.await.map_err(String::from) })
}

fn told<T: DeserializeOwned + Send + 'static>(port: u16, ask: Ask, what: &'static str, cx: &App) -> Task<Result<T, Failed>> {
    cx.background_spawn(async move {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| Failed::Lost(why.to_string()))?;
        let sent = match ask {
            Ask::Get(path) => runtime.block_on(crate::transport::get(port, path)),
            Ask::Post(path, body) => runtime.block_on(crate::transport::post(port, path, body)),
            Ask::Bare(path) => runtime.block_on(crate::transport::post_bare(port, path)),
        };
        let (status, bytes) = sent.map_err(Failed::Lost)?;
        if !(200..300).contains(&status) {
            return Err(Failed::Refused(ipc::decode::<ipc::WireError>("WireError", &bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| error.message)));
        }
        // A 2xx that does not read is Fleet's answer lost on the way: it may have done the press.
        ipc::decode::<T>(what, &bytes).map_err(|why| Failed::Lost(why.to_string()))
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
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Get("/operator/page"), "OperatorPage", cx))
    }

    /// `POST /operator/page/answer`.
    pub fn answer(&mut self, press: OperatorAnswerPress, cx: &App) -> Task<Result<OperatorAnswered, Failed>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.answer(&press));
        }
        let said = body(&press);
        match self.port(cx) {
            Some(port) => told(port, Ask::Post("/operator/page/answer", said), "OperatorAnswered", cx),
            None => Task::ready(Err(Failed::Lost("Fleet is not connected".into()))),
        }
    }

    /// `POST /operator/page/later`: last until `until`, RFC 3339, or for the rest of the local day.
    pub fn later(&mut self, id: &str, until: Option<String>, cx: &App) -> Task<Result<OperatorPage, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.later(id, until));
        }
        let said = body(&OperatorLater { id: id.to_string(), until: until.map(ipc::Instant::carried) });
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Post("/operator/page/later", said), "OperatorPage", cx))
    }

    /// `POST /operator/page/order`.
    pub fn order(&mut self, ids: Vec<String>, cx: &App) -> Task<Result<OperatorPage, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(Ok(mock.order(&ids)));
        }
        let said = body(&OperatorTurnOrder { ids });
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Post("/operator/page/order", said), "OperatorPage", cx))
    }

    /// `POST /operator/front`, which takes no body.
    pub fn in_front(&mut self, cx: &App) -> Task<Result<OperatorInFront, String>> {
        if let Wire::Mock(mock) = self {
            mock.fronted += 1;
            return Task::ready(Ok(OperatorInFront { rewriting: false }));
        }
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Bare("/operator/front"), "OperatorInFront", cx))
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
        let saved = request::<ipc::SettingsList>(port, Ask::Post("/settings/save", said), "SettingsList", cx);
        cx.background_spawn(async move { saved.await.map(|_| ()) })
    }

    /// `POST /operator/orders/remove`: a rule or a hold stops standing. Fleet sends `operator.page_changed`,
    /// which redraws the page; the mock has no event, so it answers with its page.
    pub fn forget(&mut self, id: i64, cx: &App) -> Task<Result<Option<OperatorPage>, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.forget(id).map(Some));
        }
        let said = body(&RemoveOperatorOrder { id });
        let Some(port) = self.port(cx) else { return Self::unreachable() };
        let removed = request::<OperatorOrders>(port, Ask::Post("/operator/orders/remove", said), "OperatorOrders", cx);
        cx.background_spawn(async move { removed.await.map(|_| None) })
    }

    /// `POST /operator/page/undo`: a ruling the Operator made by following him is taken back.
    pub fn undo(&mut self, id: &str, cx: &App) -> Task<Result<OperatorPage, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.undo(id));
        }
        let said = body(&OperatorUndoRuling { id: id.to_string() });
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Post("/operator/page/undo", said), "OperatorPage", cx))
    }

    /// The week's Log, from the file `digest.handled.log.path` names on this machine; the mock holds its own.
    pub fn log(&mut self, path: String, cx: &App) -> Task<Result<String, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(Ok(mock.log.clone()));
        }
        cx.background_spawn(async move { std::fs::read_to_string(&path).map_err(|why| why.to_string()) })
    }

    /// `GET /operator/talk`.
    pub fn talk(&mut self, cx: &App) -> Task<Result<OperatorTalk, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(Ok(mock.talk.clone()));
        }
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Get("/operator/talk"), "OperatorTalk", cx))
    }

    /// `POST /operator/talk`. The mock holds its answer until a scene gives one, so the wait is seen.
    pub fn say(&mut self, words: String, about: Option<String>, cx: &App) -> Option<Task<Result<OperatorSaid, String>>> {
        if let Wire::Mock(mock) = self {
            mock.heard.push((words, about));
            return None;
        }
        let said = body(&TalkToOperator { words, about });
        Some(self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Post("/operator/talk", said), "OperatorSaid", cx)))
    }

    /// `POST /sessions/dispatch`: the composer's send and a parked idea's start. The mock answers at once, and with
    /// it the `operator.dispatched` events Fleet would publish, which a live Fleet sends on its stream.
    pub fn dispatch(&mut self, job: DispatchJob, cx: &App) -> Task<Result<(JobDispatched, Vec<OperatorDispatched>), String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(Ok(mock.dispatch(job)));
        }
        let said = body(&job);
        let Some(port) = self.port(cx) else { return Self::unreachable() };
        let sent = request::<JobDispatched>(port, Ask::Post("/sessions/dispatch", said), "JobDispatched", cx);
        cx.background_spawn(async move { sent.await.map(|answer| (answer, Vec::new())) })
    }

    /// `GET /sessions/parked`: the ideas kept for later.
    pub fn parked(&mut self, cx: &App) -> Task<Result<ParkedIdeas, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(Ok(ParkedIdeas { ideas: mock.parked.clone() }));
        }
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Get("/sessions/parked"), "ParkedIdeas", cx))
    }

    /// `POST /sessions/parked/remove`: one idea goes; the answer is the ideas that are left.
    pub fn remove_parked(&mut self, id: String, cx: &App) -> Task<Result<ParkedIdeas, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.remove_parked(&id));
        }
        let said = body(&RemoveParkedIdea { id });
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Post("/sessions/parked/remove", said), "ParkedIdeas", cx))
    }
}
