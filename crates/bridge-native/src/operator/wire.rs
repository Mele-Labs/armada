//! The page's routes, each a request off the main thread on the port the Fleet connection read, or
//! answered by the walk's mock at once. `crates/ipc/operations/operator_page.toml` and its siblings.

use gpui::{App, AppContext, Entity, Task};
use ipc::operator_page::{OperatorAnswerPress, OperatorAnswered, OperatorInFront, OperatorLater, OperatorPage, OperatorTurnOrder};
use ipc::operator_talk::{OperatorOrders, OperatorSaid, OperatorTalk, RemoveOperatorOrder, TalkToOperator};
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
    Get(String),
    Post(String, String),
    Bare(String),
}

fn request<T: DeserializeOwned + Send + 'static>(port: u16, ask: Ask, what: &'static str, cx: &App) -> Task<Result<T, String>> {
    cx.background_spawn(async move {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
        let (status, bytes) = match ask {
            Ask::Get(path) => runtime.block_on(crate::transport::get(port, &path))?,
            Ask::Post(path, body) => runtime.block_on(crate::transport::post(port, &path, body))?,
            Ask::Bare(path) => runtime.block_on(crate::transport::post_bare(port, &path))?,
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
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Get("/operator/page".into()), "OperatorPage", cx))
    }

    /// `POST /operator/page/answer`.
    pub fn answer(&mut self, press: OperatorAnswerPress, cx: &App) -> Task<Result<OperatorAnswered, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.answer(&press));
        }
        let said = body(&press);
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Post("/operator/page/answer".into(), said), "OperatorAnswered", cx))
    }

    /// `POST /operator/page/later`.
    pub fn later(&mut self, id: &str, cx: &App) -> Task<Result<OperatorPage, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.later(id));
        }
        let said = body(&OperatorLater { id: id.to_string() });
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Post("/operator/page/later".into(), said), "OperatorPage", cx))
    }

    /// `POST /operator/page/order`.
    pub fn order(&mut self, ids: Vec<String>, cx: &App) -> Task<Result<OperatorPage, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(Ok(mock.order(&ids)));
        }
        let said = body(&OperatorTurnOrder { ids });
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Post("/operator/page/order".into(), said), "OperatorPage", cx))
    }

    /// `POST /operator/front`, which takes no body.
    pub fn in_front(&mut self, cx: &App) -> Task<Result<OperatorInFront, String>> {
        if let Wire::Mock(mock) = self {
            mock.fronted += 1;
            return Task::ready(Ok(OperatorInFront { rewriting: false }));
        }
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Bare("/operator/front".into()), "OperatorInFront", cx))
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
        let saved = request::<ipc::SettingsList>(port, Ask::Post("/settings/save".into(), said), "SettingsList", cx);
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
        let removed = request::<OperatorOrders>(port, Ask::Post("/operator/orders/remove".into(), said), "OperatorOrders", cx);
        cx.background_spawn(async move { removed.await.map(|_| None) })
    }

    /// The week's Log, from the file `digest.handled.log.path` names on this machine; the mock holds its own.
    pub fn log(&mut self, path: String, cx: &App) -> Task<Result<String, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(Ok(mock.log.clone()));
        }
        cx.background_spawn(async move { std::fs::read_to_string(&path).map_err(|why| why.to_string()) })
    }

    /// `GET /autopilots`: every repository's Autopilot, stopped ones included.
    pub fn autopilots(&mut self, cx: &App) -> Task<Result<ipc::AutopilotList, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(Ok(ipc::AutopilotList { autopilots: mock.autopilots.clone() }));
        }
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Get("/autopilots".into()), "AutopilotList", cx))
    }

    /// `GET /autopilot/decisions`: what the Autopilots answered that he has not looked at.
    pub fn decisions(&mut self, cx: &App) -> Task<Result<ipc::AutopilotDecisions, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(Ok(ipc::AutopilotDecisions { to_review: mock.to_review.clone() }));
        }
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Get("/autopilot/decisions".into()), "AutopilotDecisions", cx))
    }

    /// `POST /autopilots/:repository`: one repository's Autopilot starts on what he ticked.
    pub fn start_away(&mut self, manifest: &str, settings: ipc::AutopilotSettings, cx: &App) -> Task<Result<ipc::Autopilot, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.start_away(manifest, settings));
        }
        let (path, said) = (format!("/autopilots/{manifest}"), body(&settings));
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Post(path, said), "Autopilot", cx))
    }

    /// `POST /autopilots/:repository/stop`.
    pub fn stop_away(&mut self, manifest: &str, cx: &App) -> Task<Result<ipc::Autopilot, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.stop_away(manifest));
        }
        let path = format!("/autopilots/{manifest}/stop");
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Bare(path), "Autopilot", cx))
    }

    /// `POST /autopilots/:repository/picks/:pick_id/dispatch`, or `/dismiss`: a set-aside issue started or put away.
    pub fn pick(&mut self, manifest: &str, pick: &str, start: bool, cx: &App) -> Task<Result<ipc::Autopilot, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.pick(manifest, pick, start));
        }
        let path = format!("/autopilots/{manifest}/picks/{pick}/{}", if start { "dispatch" } else { "dismiss" });
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Bare(path), "Autopilot", cx))
    }

    /// `POST /autopilot/decisions/review` on Agree, or `/override` with his words on Change.
    pub fn reviewed(&mut self, id: &str, change: Option<String>, cx: &App) -> Task<Result<ipc::AutopilotDecisions, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.reviewed(id, change));
        }
        let (path, said) = match change {
            Some(text) => ("/autopilot/decisions/override", body(&ipc::OverrideAutopilotDecision { id: id.to_string(), text })),
            None => ("/autopilot/decisions/review", body(&ipc::ReviewAutopilotDecision { id: id.to_string() })),
        };
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Post(path.into(), said), "AutopilotDecisions", cx))
    }

    /// `GET /operator/talk`.
    pub fn talk(&mut self, cx: &App) -> Task<Result<OperatorTalk, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(Ok(mock.talk.clone()));
        }
        self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Get("/operator/talk".into()), "OperatorTalk", cx))
    }

    /// `POST /operator/talk`. The mock holds its answer until a scene gives one, so the wait is seen.
    pub fn say(&mut self, words: String, about: Option<String>, cx: &App) -> Option<Task<Result<OperatorSaid, String>>> {
        if let Wire::Mock(mock) = self {
            mock.heard.push((words, about));
            return None;
        }
        let said = body(&TalkToOperator { words, about });
        Some(self.port(cx).map_or_else(Self::unreachable, |port| request(port, Ask::Post("/operator/talk".into(), said), "OperatorSaid", cx)))
    }
}
