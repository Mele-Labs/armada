//! A page a Job on another Mac serves, a walk or a dev server, opened on this Mac.
//!
//! **The Mac that holds the Job** answers `/api/mac/page/<job>/<port>/<path>`, signed by a linked Mac and
//! nothing else, and asks its Fleet first whether that Job is shared and showed or serves that port. Only
//! then is the request sent on to `127.0.0.1:<port>`. An unshared Job is answered as Fleet answers a missing one.
//!
//! **The Mac that looks** opens one loopback port per page, so the page keeps an origin of its own and its
//! absolute paths still resolve, and signs each request it takes on to the other Mac. Pages are read whole
//! and a dev server's live-reload socket does not cross: he reloads by hand.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::Extension;
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};

use crate::mac_client::signer;
use crate::mac_keys::sign;
use crate::reads::{port, refusal, NOT_ANSWERING};
use crate::signing::MacId;
use crate::{fleet_client, Gateway};

/// How long one page request may take, either leg.
const PATIENCE: Duration = Duration::from_secs(30);

/// The headers a page request and its answer carry across. The rest are the transport's, or the signature's.
const CARRIED: [header::HeaderName; 5] =
    [header::CONTENT_TYPE, header::ACCEPT, header::CACHE_CONTROL, header::LOCATION, header::ETAG];

/// The client both legs use. **It follows no redirect**: a dev server's `Location` goes back to the page, and
/// a redirect followed on the signed leg would land on the other Gateway unsigned.
fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(PATIENCE)
            .build()
            .unwrap_or_default()
    })
}

/// The loopback ports this Mac opened for other Macs' pages, by Mac, Job and the page's port there.
#[derive(Clone, Default)]
pub(crate) struct Opened(Arc<Mutex<HashMap<(String, String, u16), u16>>>);

/// What opening another Mac's page came to.
pub enum PageOpen {
    /// The page's address on this Mac.
    Open(String),
    /// The Mac is asleep, by its name and since when (Unix seconds).
    Asleep { name: String, since: i64 },
    /// The other Mac refused, with its status and words as it said them.
    Refused { status: u16, body: Vec<u8> },
}

/// `GET|POST|… /api/mac/page/<job>/<port>/<path>`, signed by a linked Mac: the page, where Fleet says Job `job`
/// is shared and showed or serves `port`. Fleet's refusal is handed back as it came.
pub async fn serve(
    State(gateway): State<Gateway>,
    Extension(MacId(mac)): Extension<MacId>,
    Path(named): Path<HashMap<String, String>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let (Some(job), Some(Ok(page_port))) = (named.get("job"), named.get("port").map(|port| port.parse::<u16>())) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if let Some(refused) = crate::mac_routes::refused_frozen(&gateway, &mac) {
        return refused;
    }
    let fleet = match port(&gateway) {
        Ok(fleet) => fleet,
        Err(refused) => return refused,
    };
    let check = format!("/other_mac/page?session_id={}&port={page_port}", ipc::door::encoded(job));
    let answer = match fleet_client::send(fleet, "GET", &check, None).await {
        Ok(answer) => answer,
        Err(_) => return refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_ANSWERING),
    };
    if !(200..300).contains(&answer.status) {
        let status = StatusCode::from_u16(answer.status).unwrap_or(StatusCode::BAD_GATEWAY);
        let kind = answer.content_type.unwrap_or_else(|| String::from("application/json"));
        return (status, [(header::CONTENT_TYPE, kind)], answer.body).into_response();
    }
    // The path as it was sent, still encoded: the six pieces before it are this route's.
    let whole = uri.path_and_query().map_or(uri.path(), |whole| whole.as_str());
    let rest = whole.splitn(7, '/').nth(6).unwrap_or_default();
    let target = format!("http://127.0.0.1:{page_port}/{rest}");
    let mut request = client().request(method, target).body(body);
    for name in CARRIED.iter().filter(|name| headers.contains_key(*name)) {
        request = request.header(name, headers[name].clone());
    }
    match request.send().await {
        Ok(page) => carried_back(page).await,
        Err(_) => refusal(StatusCode::BAD_GATEWAY, "The page is not answering on that Mac."),
    }
}

/// A page's answer, with its status and [`CARRIED`] headers.
async fn carried_back(page: reqwest::Response) -> Response {
    let status = StatusCode::from_u16(page.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let kept: Vec<(header::HeaderName, HeaderValue)> =
        CARRIED.iter().filter_map(|name| page.headers().get(name).map(|value| (name.clone(), value.clone()))).collect();
    let Ok(bytes) = page.bytes().await else {
        return refusal(StatusCode::BAD_GATEWAY, "The page stopped answering on that Mac.");
    };
    let mut answer = (status, bytes).into_response();
    for (name, value) in kept {
        answer.headers_mut().insert(name, value);
    }
    answer
}

/// Open the page Job `job` of Mac `mac` serves on its port `page_port`, at `rest` (its path and query, `/`
/// first), as an address on this Mac. Fleet's `open_other_mac_page`.
pub fn open_other_mac_page(
    gateway: &Gateway,
    mac: &str,
    job: &str,
    page_port: u16,
    rest: &str,
) -> impl std::future::Future<Output = Result<PageOpen, String>> + Send + 'static {
    let (gateway, mac, job, rest) = (gateway.clone(), mac.to_string(), job.to_string(), rest.to_string());
    async move {
        let found = gateway.pairing.store.lock().unwrap().paired_mac(&mac);
        let row = match found {
            Ok(Some(row)) if row.state == store::MacState::Linked => row,
            _ => return Err(format!("This Mac is not linked to Mac {mac}.")),
        };
        if let Some(link) = crate::macs::frozen(&gateway.way_in, &row) {
            return Err(crate::macs::frozen_why(&link, &row.name));
        }
        if let Some(since) = row.asleep_since {
            return Ok(PageOpen::Asleep { name: row.name, since });
        }
        let check = format!("/api/mac/page?session_id={}&port={page_port}", ipc::door::encoded(&job));
        let came = signed_page(&gateway, &mac, &row.address, Method::GET, &check, &HeaderMap::new(), Bytes::new())
            .await
            .map_err(|why| format!("{} could not be reached: {why}", row.name))?;
        let status = came.status().as_u16();
        if !(200..300).contains(&status) {
            let body = came.bytes().await.map(|bytes| bytes.to_vec()).unwrap_or_default();
            return Ok(PageOpen::Refused { status, body });
        }
        let local = opened(&gateway, &mac, &job, page_port).await.map_err(|why| format!("This Mac could not open the page: {why}"))?;
        Ok(PageOpen::Open(format!("http://127.0.0.1:{local}{rest}")))
    }
}

/// The loopback port this Mac serves Job `job`'s page `page_port` on, opened the first time it is asked.
async fn opened(gateway: &Gateway, mac: &str, job: &str, page_port: u16) -> std::io::Result<u16> {
    let key = (mac.to_string(), job.to_string(), page_port);
    if let Some(local) = gateway.macs.pages.0.lock().unwrap().get(&key) {
        return Ok(*local);
    }
    let listener = crate::bind(0).await?;
    let local = listener.local_addr()?.port();
    let page = Page { gateway: gateway.clone(), mac: key.0.clone(), job: key.1.clone(), port: page_port };
    let app = axum::Router::new().fallback(through).with_state(page);
    let closed = gateway.macs.closed();
    tokio::spawn(async move { axum::serve(listener, app).with_graceful_shutdown(closed).await });
    gateway.macs.pages.0.lock().unwrap().insert(key, local);
    Ok(local)
}

/// One page of another Mac's, as its loopback port here serves it.
#[derive(Clone)]
struct Page {
    gateway: Gateway,
    mac: String,
    job: String,
    port: u16,
}

/// Any request to a page's loopback port, signed and sent to the Mac that holds the Job.
async fn through(State(page): State<Page>, method: Method, uri: Uri, headers: HeaderMap, body: Bytes) -> Response {
    let found = page.gateway.pairing.store.lock().unwrap().paired_mac(&page.mac);
    let Ok(Some(row)) = found.map(|row| row.filter(|row| row.state == store::MacState::Linked)) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if let Some(link) = crate::macs::frozen(&page.gateway.way_in, &row) {
        return (StatusCode::SERVICE_UNAVAILABLE, crate::macs::frozen_why(&link, &row.name)).into_response();
    }
    let rest = uri.path_and_query().map_or("/", |whole| whole.as_str());
    let target = format!("/api/mac/page/{}/{}{rest}", ipc::door::encoded(&page.job), page.port);
    match signed_page(&page.gateway, &page.mac, &row.address, method, &target, &headers, body).await {
        Ok(came) => carried_back(came).await,
        Err(_) => refusal(StatusCode::BAD_GATEWAY, "That Mac is not answering. It may be asleep."),
    }
}

/// A signed page request to paired Mac `mac`, carrying the [`CARRIED`] headers of `headers`.
async fn signed_page(
    gateway: &Gateway,
    mac: &str,
    address: &str,
    method: Method,
    target: &str,
    headers: &HeaderMap,
    body: Bytes,
) -> Result<reqwest::Response, String> {
    let (me, key) = signer(gateway, mac).await?;
    let signature = sign(&key, &me, method.as_str(), target, gateway.pairing.now(), &body)?;
    // Its door while Bonjour sees it, else its tunnel: `mac_client::signed` says why the fallback is safe.
    let ways = crate::nearby::reach(gateway, mac, address)?;
    let last = ways.len() - 1;
    for (n, (to, door)) in ways.into_iter().enumerate() {
        let leg = if to == address { client().clone() } else { door };
        let mut request = leg.request(method.clone(), format!("{to}{target}")).timeout(PATIENCE);
        for (name, value) in &signature {
            request = request.header(*name, value.clone());
        }
        for name in CARRIED.iter().filter(|name| headers.contains_key(*name)) {
            request = request.header(name, headers[name].clone());
        }
        match request.body(body.clone()).send().await {
            Err(why) if why.is_connect() && n < last => continue,
            sent => return sent.map_err(|why| why.to_string()),
        }
    }
    Err("It could not be reached.".to_string())
}
