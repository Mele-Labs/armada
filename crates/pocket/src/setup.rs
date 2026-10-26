//! `/admin/setup`: how far this Mac is along the way in Bridge walks the owner
//! through, Tailscale installed, signed in and Funnel on, and the press that
//! turns Funnel on. Bridge's Settings, Phone draws one row per step from it.
//!
//! **Read afresh at each ask**, so a step the owner finishes in Tailscale's own
//! app shows done at Bridge's next look without a press here.

use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::pairing::{address_from_status, ADDRESS_FILE, TAILSCALE};
use crate::reads::json;
use crate::Gateway;

/// What `tailscale funnel` did when asked to turn Funnel on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Funnel {
    On,
    /// The tailnet has not allowed Funnel for this Mac: the page Tailscale names
    /// to allow it. The command keeps waiting, and finishes once it is allowed.
    Approve(String),
    Refused(String),
}

/// What the Gateway asks the Mac. Injected, so a test names the answers.
#[derive(Clone)]
pub struct WayIn {
    /// Whether `pocket-address` names the address, which means the owner brought a tunnel of his own.
    pub named: Arc<dyn Fn() -> bool + Send + Sync>,
    /// What `tailscale <args>` wrote, or `None` where Tailscale is not installed. Blocking.
    pub ask: Arc<dyn Fn(&[&str]) -> Option<String> + Send + Sync>,
    /// Turns Funnel on in front of the Gateway, given the Mac's own address so the one Tailscale prints
    /// on success is not read as a page to open. Blocking.
    pub funnel: Arc<dyn Fn(Option<&str>) -> Funnel + Send + Sync>,
    /// The loopback port the Gateway listens on, which Funnel must name.
    pub port: u16,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Tailscale {
    Missing,
    SignedOut,
    Running,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Served {
    /// Nothing in front of the Gateway: the phone cannot reach it.
    None,
    /// `tailscale serve`: the phone reaches it only while Tailscale runs on the phone.
    Tailnet,
    /// Tailscale Funnel: any phone reaches it, with nothing installed on it.
    Funnel,
}

#[derive(Serialize)]
struct Setup {
    tailscale: Tailscale,
    served: Served,
    /// `pocket-address` names the address, and the steps above are not this Mac's.
    named: bool,
    /// The address the phone opens, where there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    address: Option<String>,
}

/// From `tailscale serve status --json`: whether something serves `127.0.0.1:<port>` at all, and with Funnel.
/// Read by scanning, as `address_from_status` is, because bytes are only parsed in `store` and `ipc`.
fn served_from_status(output: &str, port: u16) -> Served {
    let plain: String = output.chars().filter(|c| !c.is_whitespace()).collect();
    let proxied = [format!("\"Proxy\":\"http://127.0.0.1:{port}\""), format!("\"Proxy\":\"http://localhost:{port}\"")];
    if !proxied.iter().any(|one| plain.contains(one.as_str())) {
        return Served::None;
    }
    let funnel = plain
        .find("\"AllowFunnel\":{")
        .map(|at| &plain[at..])
        .and_then(|rest| rest.find('}').map(|end| &rest[..end]))
        .is_some_and(|allowed| allowed.contains(":443\":true"));
    if funnel {
        Served::Funnel
    } else {
        Served::Tailnet
    }
}

/// The first `https://` word in what a command wrote that is not the Mac's own address: the page it asks
/// the owner to open. Tailscale prints the Mac's own address once Funnel is on.
fn page_in(output: &str, own: Option<&str>) -> Option<String> {
    output
        .split_whitespace()
        .map(|word| word.trim_end_matches(['.', ',', ')']))
        .find(|word| word.starts_with("https://") && own.is_none_or(|own| !word.starts_with(own)))
        .map(str::to_string)
}

fn read(gateway: &Gateway) -> Setup {
    let way = &gateway.way_in;
    let named = (way.named)();
    let tailscale = match (way.ask)(&["status", "--json"]) {
        None => Tailscale::Missing,
        Some(out) if address_from_status(&out).is_ok() => Tailscale::Running,
        Some(_) => Tailscale::SignedOut,
    };
    let served = match tailscale {
        Tailscale::Running => (way.ask)(&["serve", "status", "--json"])
            .map_or(Served::None, |out| served_from_status(&out, way.port)),
        _ => Served::None,
    };
    Setup { tailscale, served, named, address: gateway.pairing.address().ok() }
}

pub async fn setup(State(gateway): State<Gateway>) -> Response {
    match tokio::task::spawn_blocking(move || read(&gateway)).await {
        Ok(setup) => json(&setup),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "The Gateway could not read this Mac's setup.").into_response(),
    }
}

#[derive(Serialize)]
struct Approve {
    approve: String,
}

pub async fn funnel(State(gateway): State<Gateway>) -> Response {
    let turn = gateway.way_in.funnel.clone();
    let pairing = gateway.pairing.clone();
    match tokio::task::spawn_blocking(move || turn(pairing.address().ok().as_deref())).await {
        Ok(Funnel::On) => StatusCode::NO_CONTENT.into_response(),
        Ok(Funnel::Approve(page)) => json(&Approve { approve: page }),
        Ok(Funnel::Refused(said)) => (StatusCode::BAD_GATEWAY, said).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Funnel could not be turned on.").into_response(),
    }
}

/// How long the press waits for `tailscale funnel` to finish or name the page to allow it.
const FUNNEL_WAIT: Duration = Duration::from_secs(10);

fn tailscale(args: &[&str]) -> Option<String> {
    TAILSCALE.iter().find_map(|program| Command::new(program).args(args).output().ok()).map(|output| {
        let mut said = String::from_utf8_lossy(&output.stdout).into_owned();
        said.push_str(&String::from_utf8_lossy(&output.stderr));
        said
    })
}

/// Runs `tailscale funnel --bg` in front of `port`. Where the tailnet has not allowed Funnel the command
/// names a page and waits, so this answers with the page and leaves it waiting, to finish once allowed.
fn turn_on_funnel(port: u16, own: Option<&str>) -> Funnel {
    let target = format!("http://127.0.0.1:{port}");
    let Some(mut child) = TAILSCALE.iter().find_map(|program| {
        Command::new(program)
            .args(["funnel", "--bg", &target])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .ok()
    }) else {
        return Funnel::Refused("Tailscale is not installed on this Mac.".to_string());
    };
    let (lines, heard) = mpsc::channel::<String>();
    for pipe in [child.stdout.take().map(|p| Box::new(p) as Box<dyn Read + Send>), child.stderr.take().map(|p| Box::new(p) as Box<dyn Read + Send>)].into_iter().flatten() {
        let lines = lines.clone();
        std::thread::spawn(move || BufReader::new(pipe).lines().map_while(Result::ok).for_each(|line| drop(lines.send(line))));
    }
    drop(lines);
    let until = Instant::now() + FUNNEL_WAIT;
    let mut said = String::new();
    loop {
        if let Some(page) = page_in(&said, own) {
            // Reaped on its own thread whenever it finishes.
            std::thread::spawn(move || drop(child.wait()));
            return Funnel::Approve(page);
        }
        let left = until.saturating_duration_since(Instant::now());
        match heard.recv_timeout(left) {
            Ok(line) => (said.push_str(&line), said.push('\n')).1,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                std::thread::spawn(move || drop(child.wait()));
                return Funnel::Refused("Tailscale did not finish turning Funnel on. Try again.".to_string());
            }
        }
    }
    match child.wait() {
        Ok(status) if status.success() => Funnel::On,
        _ => Funnel::Refused(match said.trim() {
            "" => "Tailscale could not turn Funnel on.".to_string(),
            words => format!("Tailscale could not turn Funnel on: {words}"),
        }),
    }
}

/// The Mac as it is: `pocket-address` in `machine`, and the installed Tailscale in front of `port`.
pub fn mac_way_in(machine: PathBuf, port: u16) -> WayIn {
    WayIn {
        named: Arc::new(move || machine.join(ADDRESS_FILE).is_file()),
        ask: Arc::new(tailscale),
        funnel: Arc::new(move |own| turn_on_funnel(port, own)),
        port,
    }
}

/// A Mac with no Tailscale and no `pocket-address`, for tests about something else.
#[cfg(test)]
pub(crate) fn quiet_way_in() -> WayIn {
    WayIn { named: Arc::new(|| false), ask: Arc::new(|_| None), funnel: Arc::new(|_| Funnel::Refused("no tailscale in tests".into())), port: 8443 }
}

#[cfg(test)]
mod tests {
    use axum::body::{to_bytes, Body};
    use axum::http::{Method, Request};
    use tower::ServiceExt;

    use super::*;

    const STATUS: &str = r#"{"BackendState":"Running","Self":{"DNSName":"mac.tail-1.ts.net."}}"#;
    const FUNNEL: &str = r#"{"Web":{"mac.tail-1.ts.net:443":{"Handlers":{"/":{"Proxy":"http://127.0.0.1:8443"}}}},"AllowFunnel":{"mac.tail-1.ts.net:443":true}}"#;

    fn gateway(way_in: WayIn) -> Gateway {
        Gateway { way_in, ..crate::pairing_tests::rig().gateway }
    }

    async fn call(gateway: Gateway, method: Method, path: &str) -> (StatusCode, String) {
        let answer = crate::router(gateway).oneshot(Request::builder().method(method).uri(path).body(Body::empty()).unwrap()).await.unwrap();
        let status = answer.status();
        (status, String::from_utf8(to_bytes(answer.into_body(), 1 << 16).await.unwrap().to_vec()).unwrap())
    }

    #[tokio::test]
    async fn setup_says_each_step_as_the_mac_has_it() {
        let ask = |status: Option<&'static str>, serve: Option<&'static str>| -> Arc<dyn Fn(&[&str]) -> Option<String> + Send + Sync> {
            Arc::new(move |args: &[&str]| match args {
                ["status", ..] => status.map(str::to_string),
                _ => serve.map(str::to_string),
            })
        };
        let cases = [
            (None, None, r#""tailscale":"missing","served":"none""#),
            (Some(r#"{"BackendState":"NeedsLogin"}"#), None, r#""tailscale":"signed_out","served":"none""#),
            (Some(STATUS), Some("{}"), r#""tailscale":"running","served":"none""#),
            (Some(STATUS), Some(FUNNEL), r#""tailscale":"running","served":"funnel""#),
        ];
        for (status, serve, says) in cases {
            let way_in = WayIn { ask: ask(status, serve), ..quiet_way_in() };
            let (code, body) = call(gateway(way_in), Method::GET, "/admin/setup").await;
            assert_eq!(code, StatusCode::OK);
            assert!(body.contains(says), "{body}");
            assert!(body.contains(r#""named":false"#), "{body}");
        }
    }

    #[tokio::test]
    async fn turning_funnel_on_answers_done_or_the_page_to_allow_it() {
        let on = WayIn { funnel: Arc::new(|_| Funnel::On), ..quiet_way_in() };
        assert_eq!(call(gateway(on), Method::POST, "/admin/setup/funnel").await.0, StatusCode::NO_CONTENT);
        let page = WayIn { funnel: Arc::new(|_| Funnel::Approve("https://login.example/f/funnel".into())), ..quiet_way_in() };
        let (code, body) = call(gateway(page), Method::POST, "/admin/setup/funnel").await;
        assert_eq!((code, body.as_str()), (StatusCode::OK, r#"{"approve":"https://login.example/f/funnel"}"#));
        let refused = call(gateway(quiet_way_in()), Method::POST, "/admin/setup/funnel").await;
        assert_eq!(refused.0, StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn setup_is_for_bridge_alone() {
        let (code, _) = {
            let answer = crate::router(gateway(quiet_way_in()))
                .oneshot(Request::get("/admin/setup").header("host", "mac.tail-1.ts.net").body(Body::empty()).unwrap())
                .await
                .unwrap();
            (answer.status(), ())
        };
        assert_eq!(code, StatusCode::FORBIDDEN);
    }

    const PORT: u16 = 8443;

    #[test]
    fn funnel_is_on_only_where_the_gateway_is_served_on_443_with_funnel_allowed() {
        let serve = r#"{"TCP":{"443":{"HTTPS":true}},"Web":{"mac.tail-1.ts.net:443":{"Handlers":{"/":{"Proxy":"http://127.0.0.1:8443"}}}}}"#;
        let funnel = r#"{
          "TCP": { "443": { "HTTPS": true } },
          "Web": { "mac.tail-1.ts.net:443": { "Handlers": { "/": { "Proxy": "http://127.0.0.1:8443" } } } },
          "AllowFunnel": { "mac.tail-1.ts.net:443": true }
        }"#;
        let other = r#"{"Web":{"mac.tail-1.ts.net:443":{"Handlers":{"/":{"Proxy":"http://127.0.0.1:3000"}}}},"AllowFunnel":{"mac.tail-1.ts.net:443":true}}"#;
        assert_eq!(served_from_status(serve, PORT), Served::Tailnet);
        assert_eq!(served_from_status(funnel, PORT), Served::Funnel);
        assert_eq!(served_from_status(other, PORT), Served::None);
        assert_eq!(served_from_status("{}", PORT), Served::None);
        assert_eq!(served_from_status("", PORT), Served::None);
    }

    #[test]
    fn the_page_to_allow_funnel_is_the_first_https_word() {
        let said = "Funnel is not enabled on your tailnet.\nTo enable, visit:\n\n         https://login.tailscale.com/f/funnel?node=abc123\n";
        assert_eq!(page_in(said, Some("https://mac.tail-1.ts.net")).as_deref(), Some("https://login.tailscale.com/f/funnel?node=abc123"));
        let on = "Available on the internet:\n\nhttps://mac.tail-1.ts.net/\n|-- / proxy http://127.0.0.1:8443\n";
        assert_eq!(page_in(on, Some("https://mac.tail-1.ts.net")), None);
        assert_eq!(page_in("Available on the internet: none", None), None);
    }
}
