//! A Job's terminals across his Macs. The Mac that holds the Job answers `/api/mac/terminal` and streams
//! `/api/mac/terminal/observe`, both signed, from its Fleet's `/other_mac/terminal*`, which runs nothing for a
//! Job not shared. The Mac asking follows that stream for its own Fleet with [`follow_other_mac`].
//!
//! **Nothing is queued.** A keystroke the link cannot carry is refused at once and never typed later, and a
//! stream that drops or goes quiet ends, so the terminal there shows it ended.

use std::future::Future;

use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::Extension;
use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::mac_client::signed;
use crate::mac_link::{Line, Lines};
use crate::macs::frozen;
use crate::reads::{port, refusal, NOT_ANSWERING};
use crate::signing::MacId;
use crate::{fleet_client, reads, Gateway};

/// A line with nothing on it: the other Mac reads it as the stream still being alive.
const BEATING: &str = "\n";

/// What following a terminal on another Mac came to.
pub enum Followed {
    /// Its messages, one a line as that Mac's Fleet wrote them. Closed when the stream drops or goes quiet.
    Open(mpsc::Receiver<String>),
    /// That Mac refused, with its status and bytes as it said them.
    Refused { status: u16, body: Vec<u8> },
}

/// `POST /api/mac/terminal`: the other Mac's act on a terminal of a Job of this Mac, handed to Fleet's
/// `/other_mac/terminal` as the Mac the signature names, and answered as Fleet said it.
pub async fn act(State(gateway): State<Gateway>, Extension(MacId(mac)): Extension<MacId>, body: Bytes) -> Response {
    if let Some(refused) = crate::mac_routes::refused_frozen(&gateway, &mac) {
        return refused;
    }
    let port = match port(&gateway) {
        Ok(port) => port,
        Err(refused) => return refused,
    };
    let answer = match fleet_client::send_as_mac(port, "POST", "/other_mac/terminal", Some(&body), &mac).await {
        Ok(answer) => answer,
        Err(_) => return refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_ANSWERING),
    };
    let status = StatusCode::from_u16(answer.status).unwrap_or(StatusCode::BAD_GATEWAY);
    let kind = answer.content_type.unwrap_or_else(|| String::from("application/json"));
    (status, [(header::CONTENT_TYPE, kind)], answer.body).into_response()
}

/// `GET /api/mac/terminal/observe?session_id=&run_id=`: Fleet's `/other_mac/terminal/observe`, one message a line,
/// and an empty line whenever it is quiet for a beat, as `/api/mac/live`. **Fleet's refusal is answered before
/// anything streams**, so a Job not shared is the same 404 as one that does not exist.
pub async fn observe(State(gateway): State<Gateway>, Extension(MacId(mac)): Extension<MacId>, uri: Uri) -> Response {
    if let Some(refused) = crate::mac_routes::refused_frozen(&gateway, &mac) {
        return refused;
    }
    let port = match port(&gateway) {
        Ok(port) => port,
        Err(refused) => return refused,
    };
    let path = format!("/other_mac/terminal/observe?{}", uri.query().unwrap_or_default());
    let upgraded = match fleet_client::upgrade_or_refused(port, &path).await {
        Ok(Ok(upgraded)) => upgraded,
        Ok(Err(answer)) => {
            let status = StatusCode::from_u16(answer.status).unwrap_or(StatusCode::BAD_GATEWAY);
            let kind = answer.content_type.unwrap_or_else(|| String::from("application/json"));
            return (status, [(header::CONTENT_TYPE, kind)], answer.body).into_response();
        }
        Err(_) => return refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_ANSWERING),
    };
    let lines = reads::relay(upgraded, gateway.macs.timing.beat, BEATING, |text| Some(format!("{}\n", text.trim())));
    let lines = lines.take_until(gateway.macs.closed());
    (
        [(header::CONTENT_TYPE, "application/x-ndjson"), (header::CACHE_CONTROL, "no-cache")],
        Body::from_stream(lines),
    )
        .into_response()
}

/// [`crate::Gateway`]'s answer to Fleet's `follow_other_mac`: a `/other_mac/terminal/observe?…` path, followed on
/// Mac `mac` as `/api/mac/terminal/observe?…`. Fleet follows nothing else of another Mac this way. **A frozen link
/// is refused as unsent**, as every call but the update is.
pub fn follow_other_mac(gateway: &Gateway, mac: &str, path: &str) -> impl Future<Output = Result<Followed, String>> + Send + 'static {
    let (gateway, mac, path) = (gateway.clone(), mac.to_string(), path.to_string());
    async move {
        let rest = path.strip_prefix("/other_mac/terminal/observe").filter(|rest| rest.starts_with('?'));
        let Some(rest) = rest else {
            return Err(format!("{path} is not followed on another Mac."));
        };
        let found = gateway.pairing.store.lock().unwrap().paired_mac(&mac);
        let row = match found {
            Ok(Some(row)) if row.state == store::MacState::Linked => row,
            _ => return Err(format!("This Mac is not linked to Mac {mac}.")),
        };
        if let Some(link) = frozen(&gateway.way_in, &row) {
            return Err(crate::macs::frozen_why(&link, &row.name));
        }
        let target = format!("/api/mac/terminal/observe{rest}");
        let response = signed(&gateway, &row.id, &row.address, "GET", &target, None, None)
            .await
            .map_err(|why| format!("{} could not be reached: {why}", row.name))?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.bytes().await.map(|bytes| bytes.to_vec()).unwrap_or_default();
            return Ok(Followed::Refused { status, body });
        }
        let (sender, receiver) = mpsc::channel(64);
        let (silence, closed) = (gateway.macs.timing.silence, gateway.macs.closed());
        tokio::spawn(async move {
            let mut lines = Lines::new(response);
            tokio::pin!(closed);
            loop {
                let line = tokio::select! {
                    _ = &mut closed => break,
                    line = lines.next(silence) => line,
                };
                match line {
                    Line::Text(text) if text.is_empty() => continue,
                    Line::Text(text) => {
                        if sender.send(text).await.is_err() {
                            break;
                        }
                    }
                    Line::Ended(_) => break,
                }
            }
        });
        Ok(Followed::Open(receiver))
    }
}
