//! The account's routes: the profiles, signing in, signing out, choosing which
//! profile a repository is bound to, and registering the app sign-in uses.
//! `settling`'s shape.
//!
//! **No `Resolved` extractor and no path parameter.** The account is the
//! Machine's, and nothing about it is a Job's.
//!
//! **Two routes answer a browser and not Bridge**: the page that posts the sign-in
//! app's manifest to the forge, and the address the forge sends the browser back
//! to. A browser's navigation carries no `Origin`, so the listener's doorman lets
//! it through, and each refuses anything but a state Fleet issued. Their refusals
//! are plain text, which a browser shows as words.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use ipc::{SetRepositoryProfile, SignOut};
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Accounts, Refusal};
use crate::served::Served;

/// The profiles, and the sign-in in flight.
pub(crate) async fn get_accounts<D: Accounts>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_accounts().await {
        Ok(accounts) => answer(StatusCode::OK, &accounts, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Begin signing in. **No body**: which account signs in is whoever types the
/// code at the forge.
pub(crate) async fn start_sign_in<D: Accounts>(State(served): State<Served<D>>) -> Response {
    match served.daemon().start_sign_in().await {
        Ok(accounts) => answer(StatusCode::OK, &accounts, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Begin registering the sign-in app. **No body.**
pub(crate) async fn register_sign_in_app<D: Accounts>(State(served): State<Served<D>>) -> Response {
    match served.daemon().register_sign_in_app().await {
        Ok(registration) => answer(StatusCode::OK, &registration, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?state=` on the form, and `?code=&state=` on the way back.
#[derive(Deserialize)]
pub(crate) struct AppState {
    state: String,
}

#[derive(Deserialize)]
pub(crate) struct AppCode {
    code: String,
    state: String,
}

/// What both browser routes send with every answer: the address carries the state, so
/// it goes to no other site and is kept by no cache.
fn for_a_browser(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    response
}

/// A refusal as words a browser shows.
fn said_to_a_browser(refusal: Refusal) -> Response {
    let status = StatusCode::from_u16(refusal.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let words = refusal.error().message.clone();
    for_a_browser((status, [(header::CONTENT_TYPE, "text/plain; charset=utf-8")], words).into_response())
}

/// The page that posts the manifest. 200 with the page, which carries its own policy.
pub(crate) async fn get_sign_in_app_form<D: Accounts>(
    State(served): State<Served<D>>,
    Query(asked): Query<AppState>,
) -> Response {
    match served.daemon().get_sign_in_app_form(asked.state).await {
        Ok(page) => for_a_browser((StatusCode::OK, [(header::CONTENT_TYPE, "text/html; charset=utf-8")], page).into_response()),
        Err(refusal) => said_to_a_browser(refusal),
    }
}

/// The forge sends the browser back here. 303 to the app's settings page.
pub(crate) async fn sign_in_app_registered<D: Accounts>(
    State(served): State<Served<D>>,
    Query(back): Query<AppCode>,
) -> Response {
    match served.daemon().sign_in_app_registered(back.code, back.state).await {
        Ok(settings_page) => match HeaderValue::from_str(&settings_page) {
            Ok(location) => for_a_browser((StatusCode::SEE_OTHER, [(header::LOCATION, location)]).into_response()),
            Err(_) => for_a_browser((StatusCode::OK, [(header::CONTENT_TYPE, "text/plain; charset=utf-8")], settings_page).into_response()),
        },
        Err(refusal) => said_to_a_browser(refusal),
    }
}

pub(crate) async fn sign_out<D: Accounts>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let sign_out: SignOut = match ipc::decode_body("a profile to sign out", &body) {
        Ok(sign_out) => sign_out,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().sign_out(sign_out).await {
        Ok(signed_out) => answer(StatusCode::OK, &signed_out, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn set_repository_profile<D: Accounts>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let set: SetRepositoryProfile = match ipc::decode_body("a repository and its profile", &body) {
        Ok(set) => set,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().set_repository_profile(set).await {
        Ok(accounts) => answer(StatusCode::OK, &accounts, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
