//! Where a bucket's credentials are read from: each provider's own environment variables and
//! credential files, the places its own command line tool would look. **Nothing here is kept**: a
//! secret is read into a [`Secret`] for one sync and dropped, and Kit holds none.
//!
//! The environment and the home folder are arguments, so a test names both and the machine's own
//! never leaks into it.

use std::collections::BTreeMap;
use std::path::PathBuf;

use adapter_traits::Secret;
use serde::Deserialize;

use crate::bucket_signing::Keys;

/// What a machine says about itself to a provider's tools.
pub(crate) struct Environment<'a> {
    pub(crate) var: &'a dyn Fn(&str) -> Option<String>,
    pub(crate) home: Option<PathBuf>,
}

impl Environment<'_> {
    pub(crate) fn set(&self, name: &str) -> Option<String> {
        (self.var)(name).map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
    }

    fn first_set(&self, names: &[&str]) -> Option<String> {
        names.iter().find_map(|name| self.set(name))
    }

    /// A file named by `variable`, else `relative` under the home folder.
    fn file(&self, variable: &str, relative: &str) -> Option<PathBuf> {
        match self.set(variable) {
            Some(path) => Some(PathBuf::from(path)),
            None => self.home.as_ref().map(|home| home.join(relative)),
        }
    }
}

/// The `key = value` lines under one `[section]` of an ini file, keys in lowercase.
fn section(text: &str, wanted: &str) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    let mut inside = false;
    for line in text.lines().map(str::trim).filter(|line| !line.starts_with('#') && !line.starts_with(';')) {
        if let Some(name) = line.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
            inside = name.trim() == wanted;
        } else if inside {
            if let Some((key, value)) = line.split_once('=') {
                found.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
            }
        }
    }
    found
}

fn keys_in(access: Option<String>, secret: Option<String>, session: Option<String>) -> Option<Keys> {
    Some(Keys { access: access?, secret: Secret::new(secret?), session })
}

/// The keys for an S3-shaped store: `first` and `second` name the environment's pair, and the
/// shared credentials file's profile is the fallback.
fn keys_from(env: &Environment<'_>, pair: [&str; 2], profile_file: bool) -> Option<Keys> {
    let from_env = keys_in(env.set(pair[0]), env.set(pair[1]), env.set("AWS_SESSION_TOKEN"));
    if from_env.is_some() || !profile_file {
        return from_env;
    }
    let file = env.file("AWS_SHARED_CREDENTIALS_FILE", ".aws/credentials")?;
    let text = std::fs::read_to_string(file).ok()?;
    let profile = env.set("AWS_PROFILE").unwrap_or_else(|| "default".to_string());
    let mut held = section(&text, &profile);
    keys_in(held.remove("aws_access_key_id"), held.remove("aws_secret_access_key"), held.remove("aws_session_token"))
}

/// An S3 bucket's keys, region and endpoint override.
pub(crate) fn s3(env: &Environment<'_>) -> Result<(Keys, String, Option<String>), String> {
    let keys = keys_from(env, ["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY"], true).ok_or_else(|| {
        String::from(
            "No AWS credentials: set AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY, or name a profile in ~/.aws/credentials with AWS_PROFILE",
        )
    })?;
    let region = env.first_set(&["AWS_REGION", "AWS_DEFAULT_REGION"]).or_else(|| {
        let text = std::fs::read_to_string(env.file("AWS_CONFIG_FILE", ".aws/config")?).ok()?;
        let profile = env.set("AWS_PROFILE").unwrap_or_else(|| "default".to_string());
        let name = if profile == "default" { profile } else { format!("profile {profile}") };
        section(&text, &name).remove("region")
    });
    let endpoint = env.first_set(&["AWS_ENDPOINT_URL_S3", "AWS_ENDPOINT_URL"]);
    Ok((keys, region.unwrap_or_else(|| "us-east-1".to_string()), endpoint))
}

/// An R2 bucket's keys and the address of its account's endpoint (or the override).
pub(crate) fn r2(env: &Environment<'_>) -> Result<(Keys, String), String> {
    let keys = keys_from(env, ["R2_ACCESS_KEY_ID", "R2_SECRET_ACCESS_KEY"], false)
        .or_else(|| keys_from(env, ["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY"], false))
        .ok_or_else(|| String::from("No R2 keys: set R2_ACCESS_KEY_ID and R2_SECRET_ACCESS_KEY"))?;
    let endpoint = match env.set("R2_ENDPOINT") {
        Some(endpoint) => endpoint,
        None => {
            let account = env
                .first_set(&["R2_ACCOUNT_ID", "CLOUDFLARE_ACCOUNT_ID"])
                .ok_or_else(|| String::from("No R2 account: set R2_ACCOUNT_ID"))?;
            format!("https://{account}.r2.cloudflarestorage.com")
        }
    };
    Ok((keys, endpoint))
}

/// How a Google account signs in, as its credentials file says.
pub(crate) enum Google {
    /// A service account: its address and its private key.
    Account { email: String, key: Secret<String>, token_uri: String },
    /// A person's own login, as `gcloud auth application-default login` leaves it.
    Person { client_id: String, client_secret: Secret<String>, refresh_token: Secret<String>, token_uri: String },
}

#[derive(Deserialize)]
struct GoogleFile {
    #[serde(rename = "type")]
    kind: String,
    client_email: Option<String>,
    private_key: Option<String>,
    token_uri: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
    refresh_token: Option<String>,
}

pub(crate) const GOOGLE_TOKEN_URI: &str = "https://oauth2.googleapis.com/token";

pub(crate) fn google(env: &Environment<'_>) -> Result<Google, String> {
    let file = match env.set("GOOGLE_APPLICATION_CREDENTIALS") {
        Some(path) => PathBuf::from(path),
        None => {
            let folder = match env.set("CLOUDSDK_CONFIG") {
                Some(folder) => PathBuf::from(folder),
                None => env.home.clone().unwrap_or_default().join(".config/gcloud"),
            };
            folder.join("application_default_credentials.json")
        }
    };
    let unsigned = || {
        String::from(
            "No Google credentials: set GOOGLE_APPLICATION_CREDENTIALS to a key file, or run gcloud auth application-default login",
        )
    };
    let bytes = std::fs::read(&file).map_err(|_| unsigned())?;
    let held: GoogleFile = ipc::decode("Google credentials", &bytes)
        .map_err(|_| format!("{} is not a Google credentials file", file.display()))?;
    let token_uri = held.token_uri.unwrap_or_else(|| GOOGLE_TOKEN_URI.to_string());
    match (held.kind.as_str(), held.client_email, held.private_key, held.client_id, held.client_secret, held.refresh_token) {
        ("service_account", Some(email), Some(key), ..) => Ok(Google::Account { email, key: Secret::new(key), token_uri }),
        ("authorized_user", _, _, Some(client_id), Some(client_secret), Some(refresh)) => Ok(Google::Person {
            client_id,
            client_secret: Secret::new(client_secret),
            refresh_token: Secret::new(refresh),
            token_uri,
        }),
        (kind, ..) => Err(format!("{} is a {kind:?} file this build cannot sign with", file.display())),
    }
}
