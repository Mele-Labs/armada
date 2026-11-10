//! The page's six routes, each a request off the main thread on the port the Fleet connection read, or
//! answered by the walk's mock at once. `crates/ipc/operations/operator_page.toml` and its siblings.

use gpui::{App, AppContext, Entity, Task};
use ipc::operator_page::{OperatorAnswerPress, OperatorAnswered, OperatorInFront, OperatorLater, OperatorPage, OperatorTurnOrder};
use ipc::operator_talk::{OperatorSaid, OperatorTalk, TalkToOperator};
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

fn request<T: DeserializeOwned + Send + 'static>(port: u16, ask: Ask, what: &'static str, cx: &App) -> Task<Result<T, String>> {
    cx.background_spawn(async move {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
        let (status, bytes) = match ask {
            Ask::Get(path) => runtime.block_on(crate::transport::get(port, path))?,
            Ask::Post(path, body) => runtime.block_on(crate::transport::post(port, path, body))?,
            Ask::Bare(path) => runtime.block_on(crate::transport::post_bare(port, path))?,
        };
        if !(200..300).contains(&status) {
            return Err(ipc::decode::<ipc::WireError>("WireError", &bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| error.message));
        }
        ipc::decode::<T>(what, &bytes).map_err(|why| why.to_string())
    })
}

fn body(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

impl Wire {
    fn port(&self, cx: &App) -> Option<u16> {
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
    pub fn answer(&mut self, press: OperatorAnswerPress, cx: &App) -> Task<Result<OperatorAnswered, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.answer(&press));
        }
        let said = body(&press);
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Post("/operator/page/answer", said), "OperatorAnswered", cx))
    }

    /// `POST /operator/page/later`.
    pub fn later(&mut self, id: &str, cx: &App) -> Task<Result<OperatorPage, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.later(id));
        }
        let said = body(&OperatorLater { id: id.to_string() });
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
}
