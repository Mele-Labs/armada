//! A video an agent showed, streamed to the phone from the Mac's disk.
//!
//! The phone asks once, signed, for an address (`GET /api/sessions/shown/address`); Fleet says whether the
//! job showed that file and what it is, and the Gateway answers a short-lived ticket. A `<video>` then reads
//! `GET /shown/<ticket>` with ranges, so it can seek.
//!
//! **`/shown/<ticket>` is outside the signed layer on purpose**: a video element sends no custom header, so
//! it cannot sign. The ticket stands in for the signature. It is 32 random bytes, held in memory only,
//! good for 15 minutes, and bound to the device that asked: the request is refused unless that device is
//! still paired, so unpairing a phone ends its streams. The ticket names a path Fleet already vouched for,
//! and the request carries no path of its own.

use std::io::SeekFrom;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::Extension;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use futures_util::{stream, Stream};
use hyper::body::Bytes;
use ipc::ShownVideo;
use serde::Serialize;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::reads::{json, port, refusal, NOT_ANSWERING};
use crate::signing::DeviceId;
use crate::{fleet_client, webpush, Gateway};

const LIFE: i64 = 15 * 60;
const MOST_LIVE: usize = 64;
const CHUNK: u64 = 64 * 1024;

const NOT_A_VIDEO: &str = "Armada did not answer with a video for this file.";
const NO_ADDRESS: &str = "Armada could not make an address for this video. Try again.";

struct Ticket {
    name: String,
    path: String,
    media_type: String,
    device: String,
    expires: i64,
}

/// The live tickets, oldest first. Nothing here is written to disk: a restart ends every stream.
#[derive(Clone, Default)]
pub(crate) struct Tickets(Arc<Mutex<Vec<Ticket>>>);

impl Tickets {
    fn mint(&self, video: ShownVideo, device: String, now: i64) -> Result<String, String> {
        let name = B64.encode(webpush::random::<32>()?);
        let mut live = self.0.lock().unwrap();
        live.retain(|ticket| ticket.expires > now);
        while live.len() >= MOST_LIVE {
            live.remove(0);
        }
        live.push(Ticket { name: name.clone(), path: video.path, media_type: video.media_type, device, expires: now + LIFE });
        Ok(name)
    }

    /// The path, type and device of a ticket not yet expired.
    fn live(&self, name: &str, now: i64) -> Option<(String, String, String)> {
        let live = self.0.lock().unwrap();
        let found = live.iter().find(|ticket| ticket.name == name && ticket.expires > now)?;
        Some((found.path.clone(), found.media_type.clone(), found.device.clone()))
    }
}

#[derive(Serialize)]
struct Minted {
    address: String,
    media_type: String,
}

/// `GET /api/sessions/shown/address`: Fleet's `GET /sessions/shown` for the same query, and for a video, a
/// ticket. Fleet's refusal reaches the phone as it came.
pub async fn address(
    State(gateway): State<Gateway>,
    Extension(DeviceId(device)): Extension<DeviceId>,
    uri: Uri,
) -> Response {
    let port = match port(&gateway) {
        Ok(port) => port,
        Err(refused) => return refused,
    };
    let target = format!("/sessions/shown?{}", uri.query().unwrap_or_default());
    let answer = match fleet_client::send(port, "GET", &target, None).await {
        Ok(answer) => answer,
        Err(_) => return refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_ANSWERING),
    };
    if answer.status != 200 {
        let status = StatusCode::from_u16(answer.status).unwrap_or(StatusCode::BAD_GATEWAY);
        let kind = answer.content_type.unwrap_or_else(|| String::from("application/json"));
        return (status, [(header::CONTENT_TYPE, kind)], answer.body).into_response();
    }
    let Ok(video) = ipc::decode::<ShownVideo>("a shown video", &answer.body) else {
        return refusal(StatusCode::UNPROCESSABLE_ENTITY, NOT_A_VIDEO);
    };
    let media_type = video.media_type.clone();
    match gateway.pairing.shown.mint(video, device, gateway.pairing.now()) {
        Ok(name) => json(&Minted { address: format!("/shown/{name}"), media_type }),
        Err(_) => refusal(StatusCode::INTERNAL_SERVER_ERROR, NO_ADDRESS),
    }
}

/// What a `Range` header asks for, against a file of `size` bytes.
#[derive(Debug, PartialEq, Eq)]
enum Wanted {
    Whole,
    /// First and last byte, both included and both inside the file.
    Part(u64, u64),
    Unsatisfiable,
}

/// One `bytes=a-b`, `bytes=a-` or `bytes=-n`. Several ranges, another unit, or a header that does not parse
/// is the whole file, which RFC 9110 allows a server to do with a range it does not honour.
fn wanted(range: Option<&str>, size: u64) -> Wanted {
    let Some(spec) = range.and_then(|value| value.trim().strip_prefix("bytes=")) else {
        return Wanted::Whole;
    };
    let Some((from, to)) = spec.split_once('-').filter(|_| !spec.contains(',')) else {
        return Wanted::Whole;
    };
    let (from, to) = (from.trim(), to.trim());
    if from.is_empty() {
        return match to.parse::<u64>() {
            Ok(0) => Wanted::Unsatisfiable,
            Ok(_) if size == 0 => Wanted::Unsatisfiable,
            Ok(last) => Wanted::Part(size.saturating_sub(last), size - 1),
            Err(_) => Wanted::Whole,
        };
    }
    let Ok(start) = from.parse::<u64>() else {
        return Wanted::Whole;
    };
    let end = match to.is_empty() {
        true => None,
        false => match to.parse::<u64>() {
            Ok(end) => Some(end),
            Err(_) => return Wanted::Whole,
        },
    };
    if end.is_some_and(|end| end < start) {
        return Wanted::Whole;
    }
    if start >= size {
        return Wanted::Unsatisfiable;
    }
    Wanted::Part(start, end.map_or(size - 1, |end| end.min(size - 1)))
}

/// `length` bytes of `file` from where it is positioned, a chunk at a time. A file that shrank under the
/// stream ends it short, and the connection closes on the length it promised.
fn chunks(file: File, length: u64) -> impl Stream<Item = std::io::Result<Bytes>> {
    stream::unfold((file, length), |(mut file, left)| async move {
        if left == 0 {
            return None;
        }
        let mut buffer = vec![0u8; left.min(CHUNK) as usize];
        match file.read(&mut buffer).await {
            Ok(0) => None,
            Ok(read) => {
                buffer.truncate(read);
                Some((Ok(Bytes::from(buffer)), (file, left - read as u64)))
            }
            Err(why) => Some((Err(why), (file, 0))),
        }
    })
}

/// `GET /shown/<ticket>`. Unknown, expired, an unpaired device, or a file gone since minting is a bare 404.
pub async fn serve(State(gateway): State<Gateway>, Path(name): Path<String>, headers: HeaderMap) -> Response {
    let pairing = &gateway.pairing;
    let Some((path, media_type, device)) = pairing.shown.live(&name, pairing.now()) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let paired = pairing.store.lock().unwrap().paired_device(&device);
    if !matches!(paired, Ok(Some(_))) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Ok(mut file) = File::open(&path).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let size = match file.metadata().await {
        Ok(meta) if meta.is_file() => meta.len(),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    let range = headers.get(header::RANGE).and_then(|value| value.to_str().ok());
    let (status, from, length, content_range) = match wanted(range, size) {
        Wanted::Whole => (StatusCode::OK, 0, size, None),
        Wanted::Part(first, last) => {
            (StatusCode::PARTIAL_CONTENT, first, last - first + 1, Some(format!("bytes {first}-{last}/{size}")))
        }
        Wanted::Unsatisfiable => {
            return (StatusCode::RANGE_NOT_SATISFIABLE, [(header::CONTENT_RANGE, format!("bytes */{size}"))]).into_response();
        }
    };
    if from > 0 && file.seek(SeekFrom::Start(from)).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let mut answer = (
        status,
        [
            (header::ACCEPT_RANGES, "bytes".to_string()),
            (header::CONTENT_TYPE, media_type),
            (header::CONTENT_LENGTH, length.to_string()),
            (header::CACHE_CONTROL, "no-store".to_string()),
            (header::REFERRER_POLICY, "no-referrer".to_string()),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_string()),
        ],
        Body::from_stream(chunks(file, length)),
    )
        .into_response();
    if let Some(content_range) = content_range.and_then(|value| value.parse().ok()) {
        answer.headers_mut().insert(header::CONTENT_RANGE, content_range);
    }
    answer
}
