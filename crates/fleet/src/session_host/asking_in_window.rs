//! `ask_in_window`: a session asks the person about a place on the page its walk window shows.
//! `docs/concepts/session.md`, *Asking in the window*.
//!
//! **Only about the walk.** The owner, 11 Oct 2026: a question about the walk opens in the walk,
//! and one that is not should wait. So the address must sit on the origin of a window the session
//! showed, and anything else is refused toward `AskUserQuestion`, which Fleet keeps whole while he walks.
//!
//! **It does not wait.** The row is kept and the call answers; the open question is read back off
//! the thread as a waiting item (`waiting.rs`), and `answer_waiting` settles it.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{AskInWindow, SessionRecord, SessionRow, WaitingOption, WindowPlace};

use crate::daemon::Fleet;

const WINDOW_ASK_UNNAMED: &str = "fleet.window_ask_unnamed";
const WINDOW_ASK_EMPTY: &str = "fleet.window_ask_empty";
const WINDOW_ASK_NOT_WALKED: &str = "fleet.window_ask_not_walked";

/// The id a question's waiting item goes by, from its row's.
pub(crate) const WINDOW_ASK_PREFIX: &str = "window_ask:";

/// Scheme, host and port, lowercased, with the scheme's own port dropped: `http://localhost:5173`.
/// `None` for anything but an `http` or `https` address with a host.
pub(crate) fn origin_of(url: &str) -> Option<String> {
    let lowered = url.trim();
    let (scheme, rest) = lowered.split_once("://")?;
    let scheme = scheme.to_ascii_lowercase();
    let default = match scheme.as_str() {
        "http" => ":80",
        "https" => ":443",
        _ => return None,
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = authority
        .rsplit('@')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if host.is_empty() || host.starts_with(':') {
        return None;
    }
    let host = host.strip_suffix(default).unwrap_or(&host);
    Some(format!("{scheme}://{host}"))
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
    pub(crate) async fn window_asked(
        &self,
        caller: Option<api::Caller>,
        ask: AskInWindow,
    ) -> Result<SessionRecord, Refusal> {
        let question = ask.question.trim().to_string();
        let selector = ask.selector.trim().to_string();
        let url = ask.url.trim().to_string();
        if question.is_empty() {
            return Err(self.hosted_refusal(WINDOW_ASK_EMPTY, "a question needs its words"));
        }
        if selector.is_empty() {
            return Err(self.hosted_refusal(
                WINDOW_ASK_EMPTY,
                "a question needs a `selector` for the place on the page it is about",
            ));
        }
        let options: Vec<WaitingOption> = ask
            .options
            .into_iter()
            .map(|one| WaitingOption {
                label: one.label.trim().to_string(),
                description: one
                    .description
                    .map(|said| said.trim().to_string())
                    .filter(|said| !said.is_empty()),
            })
            .collect();
        if options.iter().any(|one| one.label.is_empty()) {
            return Err(self.hosted_refusal(WINDOW_ASK_EMPTY, "every option needs a label"));
        }
        let Some(origin) = origin_of(&url) else {
            return Err(self.hosted_refusal(
                WINDOW_ASK_NOT_WALKED,
                "an `http` or `https` address on the page your walk window shows",
            ));
        };
        let id = caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| {
                ask.session_id
                    .map(|named| named.as_str().trim().to_string())
            })
            .filter(|id| !id.is_empty())
            .ok_or_else(|| self.hosted_refusal(WINDOW_ASK_UNNAMED, "a job to ask it for"))?;
        let window = {
            let store = self.store().lock().await;
            let session = store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(WINDOW_ASK_UNNAMED, "a job Fleet knows"))?;
            self.ledger_row(&store, &session)?
                .attachments
                .into_iter()
                .filter(|one| {
                    one.kind == "artifact"
                        && one.detail.get("form").map(String::as_str) == Some("window")
                        && one.state == ipc::AttachmentState::Standing
                        && origin_of(&one.target).as_deref() == Some(origin.as_str())
                })
                .max_by(|a, b| a.changed_at.cmp(&b.changed_at))
                .map(|one| one.target)
        };
        // A window of several takes is on the origin of any of them, not only its first's.
        let window = match window {
            Some(window) => Some(window),
            None => self.rows_of(&id).await.ok().and_then(|rows| {
                rows.into_iter().rev().find_map(|row| match row {
                    SessionRow::Window { url, variations, .. }
                        if variations.iter().any(|take| origin_of(&take.url).as_deref() == Some(origin.as_str())) =>
                    {
                        Some(url)
                    }
                    _ => None,
                })
            }),
        };
        let Some(window) = window else {
            return Err(self.hosted_refusal(
                WINDOW_ASK_NOT_WALKED,
                &format!(
                    "no window this job showed is on {origin}. Show the page with show_window \
                     first. A question that is not about the walk is asked with AskUserQuestion, which Fleet keeps whole in his Needs you list while he walks"
                ),
            ));
        };
        let text = ask
            .text
            .map(|said| said.trim().to_string())
            .filter(|said| !said.is_empty());
        self.row_put(
            &id,
            SessionRow::WindowAsk {
                id: self.row_id(&id),
                at: self.instant(),
                question,
                options,
                place: WindowPlace {
                    url,
                    selector,
                    text,
                },
                window,
                answer: None,
            },
        )
        .await;
        let record = {
            let store = self.store().lock().await;
            let session = store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(WINDOW_ASK_UNNAMED, "a job Fleet knows"))?;
            self.ledger_row(&store, &session)?
        };
        self.publish(ipc::Event::SessionChanged(record.clone()));
        Ok(record)
    }

    /// Keep how a question asked in the window was answered, on its row. **Best effort**, as
    /// `row_put` is: the message to the session is what carries the answer.
    pub(crate) async fn window_ask_answered(&self, session: &str, row_id: &str, answer: String) {
        let Ok(rows) = self.rows_of(session).await else {
            return;
        };
        let found = rows.into_iter().find_map(|row| match row {
            SessionRow::WindowAsk {
                id,
                at,
                question,
                options,
                place,
                window,
                ..
            } if id == row_id => Some(SessionRow::WindowAsk {
                id,
                at,
                question,
                options,
                place,
                window,
                answer: Some(answer.clone()),
            }),
            _ => None,
        });
        if let Some(row) = found {
            self.row_put(session, row).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::origin_of;

    #[test]
    fn an_origin_is_scheme_host_and_a_port_that_is_not_the_schemes_own() {
        assert_eq!(
            origin_of("http://localhost:5173/findings?x=1").as_deref(),
            Some("http://localhost:5173")
        );
        assert_eq!(
            origin_of("HTTPS://Example.com:443/a").as_deref(),
            Some("https://example.com")
        );
        assert_eq!(
            origin_of("http://user@example.com:80").as_deref(),
            Some("http://example.com")
        );
        assert_ne!(
            origin_of("http://localhost:5173/"),
            origin_of("http://localhost:6006/")
        );
        assert_eq!(origin_of("file:///etc/hosts"), None);
        assert_eq!(origin_of("http:///nohost"), None);
    }
}
