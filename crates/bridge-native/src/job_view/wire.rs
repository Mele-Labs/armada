//! What the open Job's head and message box ask of Fleet, and how the answer comes back to the view.
//!
//! Every act is asked off the main thread; `done` runs on it with Fleet's answer, so a view puts
//! right what it drew first (`optimistic`) with the reason Fleet gave. With no Fleet, as on mock
//! data, the act is answered at once, as a Fleet that took it would.

use gpui::{Context, Window};

use super::JobView;

pub(super) type Answer = Result<Vec<u8>, String>;

/// What `done` runs.
pub(super) type Done = Box<dyn FnOnce(&mut JobView, Answer, &mut Window, &mut Context<JobView>)>;

/// A path segment or a query value, percent-encoded (`encodeURIComponent`).
pub(crate) fn encoded(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Fleet's own reason for a refusal, as a sentence the person reads.
pub(super) fn reason(status: u16, bytes: &[u8]) -> String {
    refusal(status, bytes).0
}

/// Fleet's reason for a refusal, with its code where the body carries one.
pub(super) fn refusal(status: u16, bytes: &[u8]) -> (String, Option<String>) {
    ipc::decode::<ipc::WireError>("WireError", bytes).map_or_else(|_| (format!("Fleet answered {status}"), None), |error| (error.message, Some(error.code)))
}

#[derive(Clone, Copy)]
enum Verb {
    Get,
    Post,
    PostBare,
}

impl JobView {
    pub(super) fn port(&self, cx: &gpui::App) -> Option<u16> {
        self.fleet.as_ref().and_then(|fleet| fleet.read(cx).held().port())
    }

    /// POST a body to `path`.
    pub(super) fn post(&mut self, path: String, body: Result<String, ipc::Unencodable>, window: &mut Window, cx: &mut Context<Self>, done: Done) {
        match body {
            Ok(body) => self.ask(Verb::Post, path, Some(body), window, cx, done),
            Err(why) => {
                let said = format!("{why:?}");
                done(self, Err(said), window, cx);
            }
        }
    }

    /// POST with no body, as a route that takes none is asked.
    pub(super) fn post_bare(&mut self, path: String, window: &mut Window, cx: &mut Context<Self>, done: Done) {
        self.ask(Verb::PostBare, path, None, window, cx, done);
    }

    pub(super) fn get(&mut self, path: String, window: &mut Window, cx: &mut Context<Self>, done: Done) {
        self.ask(Verb::Get, path, None, window, cx, done);
    }

    fn ask(&mut self, verb: Verb, path: String, body: Option<String>, window: &mut Window, cx: &mut Context<Self>, done: Done) {
        self.refused_code = None;
        if self.fleet.is_none() {
            done(self, Ok(Vec::new()), window, cx);
            return;
        }
        let Some(port) = self.port(cx) else {
            done(self, Err("Fleet is not connected".into()), window, cx);
            return;
        };
        cx.spawn_in(window, async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| (why.to_string(), None))?;
                    let (status, bytes) = runtime
                        .block_on(async {
                            match verb {
                                Verb::Get => crate::transport::get(port, &path).await,
                                Verb::Post => crate::transport::post(port, &path, body.unwrap_or_default()).await,
                                Verb::PostBare => crate::transport::post_bare(port, &path).await,
                            }
                        })
                        .map_err(|why| (why, None))?;
                    if (200..300).contains(&status) {
                        Ok(bytes)
                    } else {
                        Err(refusal(status, &bytes))
                    }
                })
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                let answer = answer.map_err(|(why, code)| {
                    this.refused_code = code;
                    why
                });
                done(this, answer, window, cx);
            });
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_value_is_percent_encoded_as_electron_encodes_it() {
        assert_eq!(encoded("a query/with slashes"), "a%20query%2Fwith%20slashes");
        assert_eq!(encoded("sess-comp_1.rs"), "sess-comp_1.rs");
    }
}
