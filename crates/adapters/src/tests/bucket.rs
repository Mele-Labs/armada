//! A bucket's client: the version 4 signature against the provider's published example, where
//! each provider's credentials are looked for, and the sends against a socket on this machine.
//!
//! `bucket_test_key.pem` is a key made for these tests alone. It signs nothing that is real.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use adapter_traits::{Bucket, BucketError, Secret};
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;

use super::store::serve;
use crate::bucket::{BucketAddress, ObjectBucket};
use crate::bucket_credentials::Environment;
use crate::bucket_signing::{self, Keys, Signing};

const TEST_KEY: &str = include_str!("bucket_test_key.pem");
const NOW: i64 = 1_700_000_000;

fn now() -> i64 {
    NOW
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bucket-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn address(text: &str) -> BucketAddress {
    BucketAddress::parse(text).expect("an address")
}

fn open(address: &BucketAddress, vars: &[(&str, &str)], home: &Path) -> Result<ObjectBucket, BucketError> {
    let vars: BTreeMap<String, String> = vars.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    let var = |name: &str| vars.get(name).cloned();
    ObjectBucket::open_in(address, &Environment { var: &var, home: Some(home.to_path_buf()) }, Duration::from_secs(5), now)
}

#[test]
fn the_instant_is_written_as_the_provider_writes_it() {
    assert_eq!(bucket_signing::stamps(1_369_353_600), ("20130524T000000Z".to_string(), "20130524".to_string()));
    assert_eq!(bucket_signing::stamps(NOW).0, "20231114T221320Z");
    assert_eq!(bucket_signing::stamps(951_782_400).1, "20000229", "a leap day");
}

#[test]
fn a_request_is_signed_as_the_providers_published_example_is() {
    // The "GET Object" example of the S3 documentation, signature version 4.
    let keys = Keys {
        access: "AKIAIOSFODNN7EXAMPLE".to_string(),
        secret: Secret::new("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
        session: None,
    };
    let headers = bucket_signing::sigv4(
        &keys,
        &Signing {
            method: "GET",
            host: "examplebucket.s3.amazonaws.com",
            path: "/test.txt",
            body: b"",
            extra: &[("Range", "bytes=0-9")],
            region: "us-east-1",
            epoch_seconds: 1_369_353_600,
        },
    );
    let authorization = &headers.iter().find(|(name, _)| name == "authorization").expect("signed").1;
    assert_eq!(
        authorization,
        "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request, \
         SignedHeaders=host;range;x-amz-content-sha256;x-amz-date, \
         Signature=f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41"
    );
}

#[test]
fn a_session_token_is_sent_and_signed() {
    let keys = Keys { access: "AK".into(), secret: Secret::new("sk".into()), session: Some("tok".into()) };
    let request = Signing { method: "PUT", host: "h", path: "/b/k", body: b"x", extra: &[], region: "auto", epoch_seconds: NOW };
    let headers = bucket_signing::sigv4(&keys, &request);
    assert!(headers.contains(&("x-amz-security-token".to_string(), "tok".to_string())));
    let authorization = &headers.iter().find(|(name, _)| name == "authorization").unwrap().1;
    assert!(authorization.contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date;x-amz-security-token,"), "{authorization}");
}

#[test]
fn only_a_bucket_address_with_a_provider_and_a_sound_name_is_one() {
    let shop = address("s3://shop-runs/armada/");
    assert_eq!(shop.key("runs/a.toml"), "armada/runs/a.toml");
    assert_eq!(address("gs://shop-runs").key("runs/a.toml"), "runs/a.toml");
    assert_eq!(address("r2://shop.runs/a/b/").key("x"), "a/b/x");
    for bad in ["", "shop", "https://shop-runs/x", "s3://", "s3://ab/x", "s3://Shop/x", "s3://shop runs/x", "s3://shop-runs/../x", "s3://shop-runs/a?b", "azure://shop-runs/x"] {
        assert!(BucketAddress::parse(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn no_credentials_is_unsigned_for_each_provider_and_sends_nothing() {
    let home = scratch("none");
    for text in ["s3://shop-runs/x", "r2://shop-runs/x", "gs://shop-runs/x"] {
        let said = open(&address(text), &[], &home).err().expect("refused");
        assert!(matches!(said, BucketError::Unsigned(_)), "{text}: {said:?}");
    }
}

#[test]
fn s3_keys_come_from_the_environment_or_a_named_profile_in_the_shared_file() {
    let home = scratch("aws");
    std::fs::create_dir_all(home.join(".aws")).unwrap();
    std::fs::write(
        home.join(".aws/credentials"),
        "[default]\naws_access_key_id = AKDEFAULT\naws_secret_access_key = sdefault\n\n[work]\naws_access_key_id = AKWORK\naws_secret_access_key = swork\naws_session_token = twork\n",
    )
    .unwrap();
    std::fs::write(home.join(".aws/config"), "[default]\nregion = eu-west-1\n\n[profile work]\nregion = ap-south-1\n").unwrap();

    let (keys, region, endpoint) = crate::bucket_credentials::s3(&env(&[("AWS_PROFILE", "work")], &home)).expect("a profile");
    assert_eq!((keys.access.as_str(), keys.session.as_deref(), region.as_str(), endpoint), ("AKWORK", Some("twork"), "ap-south-1", None));

    let (keys, region, _) = crate::bucket_credentials::s3(&env(&[], &home)).expect("the default");
    assert_eq!((keys.access.as_str(), region.as_str()), ("AKDEFAULT", "eu-west-1"));

    let (keys, region, endpoint) = crate::bucket_credentials::s3(&env(
        &[("AWS_ACCESS_KEY_ID", "AKENV"), ("AWS_SECRET_ACCESS_KEY", "senv"), ("AWS_REGION", "us-west-2"), ("AWS_ENDPOINT_URL", "http://127.0.0.1:9")],
        &home,
    ))
    .expect("the environment");
    assert_eq!((keys.access.as_str(), region.as_str(), endpoint.as_deref()), ("AKENV", "us-west-2", Some("http://127.0.0.1:9")));
}

fn env<'a>(vars: &'a [(&'a str, &'a str)], home: &Path) -> Environment<'static> {
    let vars: BTreeMap<String, String> = vars.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    let var: &'static dyn Fn(&str) -> Option<String> = Box::leak(Box::new(move |name: &str| vars.get(name).cloned()));
    Environment { var, home: Some(home.to_path_buf()) }
}

#[test]
fn r2_needs_its_account_and_falls_back_to_the_aws_pair_for_keys() {
    let home = scratch("r2");
    let said = crate::bucket_credentials::r2(&env(&[("R2_ACCESS_KEY_ID", "a"), ("R2_SECRET_ACCESS_KEY", "b")], &home)).err().expect("no account");
    assert!(said.contains("R2_ACCOUNT_ID"), "{said}");
    let (keys, endpoint) = crate::bucket_credentials::r2(&env(
        &[("AWS_ACCESS_KEY_ID", "a"), ("AWS_SECRET_ACCESS_KEY", "b"), ("CLOUDFLARE_ACCOUNT_ID", "acct1")],
        &home,
    ))
    .expect("keys");
    assert_eq!((keys.access.as_str(), endpoint.as_str()), ("a", "https://acct1.r2.cloudflarestorage.com"));
}

const KEYS: [(&str, &str); 4] = [
    ("AWS_ACCESS_KEY_ID", "AKTEST"),
    ("AWS_SECRET_ACCESS_KEY", "secret"),
    ("AWS_REGION", "us-east-1"),
    ("AWS_ENDPOINT_URL_S3", ""),
];

fn s3_at(url: &str) -> ObjectBucket {
    let home = scratch("s3");
    let mut vars = KEYS.to_vec();
    vars[3] = ("AWS_ENDPOINT_URL_S3", url);
    open(&address("s3://shop-runs/armada"), &vars, &home).expect("opened")
}

#[test]
fn an_object_is_put_and_read_at_its_path_signed_with_the_hosts_own_name() {
    let (url, server) = serve(vec![(200, ""), (200, "version = 1\n")]);
    let bucket = s3_at(&url);
    bucket.put("runs/acme--shop.toml", b"version = 1\n").expect("written");
    assert_eq!(bucket.get("runs/acme--shop.toml").expect("read"), Some(b"version = 1\n".to_vec()));
    let seen = server.join().unwrap();
    assert_eq!((seen[0].method.as_str(), seen[0].target.as_str()), ("PUT", "/shop-runs/armada/runs/acme--shop.toml"));
    assert_eq!(seen[0].body, b"version = 1\n");
    assert_eq!((seen[1].method.as_str(), seen[1].target.as_str()), ("GET", "/shop-runs/armada/runs/acme--shop.toml"));
    let host = url.trim_start_matches("http://");
    let again = bucket_signing::sigv4(
        &Keys { access: "AKTEST".into(), secret: Secret::new("secret".into()), session: None },
        &Signing { method: "PUT", host, path: &seen[0].target, body: &seen[0].body, extra: &[], region: "us-east-1", epoch_seconds: NOW },
    );
    let sent = seen[0].header("authorization").expect("signed");
    assert_eq!(sent, again.iter().find(|(name, _)| name == "authorization").unwrap().1, "the server saw what a signer makes of what it received");
    assert_eq!(seen[0].header("x-amz-date"), Some("20231114T221320Z"));
}

#[test]
fn a_missing_object_is_none_a_forbidden_one_is_refused_and_a_failing_one_is_unreachable() {
    let (url, server) = serve(vec![(404, "<Error><Code>NoSuchKey</Code></Error>"), (403, "<Error><Code>AccessDenied</Code></Error>"), (503, "slow down")]);
    let bucket = s3_at(&url);
    assert_eq!(bucket.get("runs/x.toml").expect("none"), None);
    let forbidden = bucket.get("runs/x.toml").expect_err("refused");
    assert!(matches!(&forbidden, BucketError::Refused(why) if why.contains("AccessDenied") && why.contains("list")), "{forbidden:?}");
    assert!(bucket.put("runs/x.toml", b"x").expect_err("down").is_unreachable());
    server.join().unwrap();
}

#[test]
fn a_bucket_nothing_listens_for_is_unreachable() {
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    assert!(s3_at(&url).get("runs/x.toml").expect_err("down").is_unreachable());
}

fn pem_json(token_uri: &str) -> String {
    let key = TEST_KEY.replace('\n', "\\n");
    format!(r#"{{"type":"service_account","client_email":"armada@test.iam","private_key":"{key}","token_uri":"{token_uri}"}}"#)
}

fn der() -> Vec<u8> {
    let body: String = TEST_KEY.lines().filter(|line| !line.starts_with("-----")).collect();
    STANDARD.decode(body).expect("a PEM")
}

#[test]
fn a_service_account_buys_a_token_with_a_signed_assertion_and_sends_it_as_the_bearer() {
    let (url, server) = serve(vec![(200, r#"{"access_token":"ya29.test","expires_in":3600}"#), (200, "version = 1\n")]);
    let home = scratch("gcs");
    let file = home.join("account.json");
    std::fs::write(&file, pem_json(&format!("{url}/token"))).unwrap();
    let vars = [("GOOGLE_APPLICATION_CREDENTIALS", file.to_str().unwrap()), ("STORAGE_EMULATOR_HOST", url.as_str())];

    let bucket = open(&address("gs://shop-runs/armada"), &vars, &home).expect("opened");
    assert_eq!(bucket.get("runs/acme--shop.toml").expect("read"), Some(b"version = 1\n".to_vec()));

    let seen = server.join().unwrap();
    assert_eq!((seen[0].method.as_str(), seen[0].target.as_str()), ("POST", "/token"));
    let form = String::from_utf8(seen[0].body.clone()).unwrap();
    assert!(form.contains("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer"), "{form}");
    let assertion = form.split("assertion=").nth(1).expect("an assertion");
    let parts: Vec<&str> = assertion.split('.').collect();
    assert_eq!(parts.len(), 3);
    let claims = String::from_utf8(URL_SAFE_NO_PAD.decode(parts[1]).unwrap()).unwrap();
    assert!(claims.contains(r#""iss":"armada@test.iam""#) && claims.contains(&format!(r#""aud":"{url}/token""#)), "{claims}");
    assert!(claims.contains(&format!(r#""iat":{NOW}"#)), "{claims}");
    let pair = ring::signature::RsaKeyPair::from_pkcs8(&der()).unwrap();
    let signature = URL_SAFE_NO_PAD.decode(parts[2]).unwrap();
    ring::signature::UnparsedPublicKey::new(&ring::signature::RSA_PKCS1_2048_8192_SHA256, pair.public().as_ref())
        .verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &signature)
        .expect("the assertion verifies under the account's public key");

    assert_eq!((seen[1].method.as_str(), seen[1].target.as_str()), ("GET", "/shop-runs/armada/runs/acme--shop.toml"));
    assert_eq!(seen[1].header("authorization"), Some("Bearer ya29.test"));
}

#[test]
fn a_persons_own_login_refreshes_its_token_and_a_refused_token_is_refused() {
    let (url, server) = serve(vec![(200, r#"{"access_token":"ya29.person"}"#), (401, r#"{"error":"invalid_grant"}"#)]);
    let home = scratch("gcs-person");
    let file = home.join("adc.json");
    std::fs::write(
        &file,
        format!(r#"{{"type":"authorized_user","client_id":"cid","client_secret":"csecret","refresh_token":"rtok","token_uri":"{url}/token"}}"#),
    )
    .unwrap();
    let vars = [("GOOGLE_APPLICATION_CREDENTIALS", file.to_str().unwrap()), ("STORAGE_EMULATOR_HOST", url.as_str())];
    assert!(open(&address("gs://shop-runs"), &vars, &home).is_ok());
    let said = open(&address("gs://shop-runs"), &vars, &home).err().expect("refused");
    assert!(matches!(said, BucketError::Refused(ref why) if why.contains("invalid_grant")), "{said:?}");
    let seen = server.join().unwrap();
    let form = String::from_utf8(seen[0].body.clone()).unwrap();
    assert!(form.contains("grant_type=refresh_token") && form.contains("refresh_token=rtok") && form.contains("client_id=cid"), "{form}");
}

#[test]
fn a_credentials_file_this_build_cannot_sign_with_is_named_and_not_guessed_at() {
    let home = scratch("gcs-odd");
    let file = home.join("odd.json");
    std::fs::write(&file, r#"{"type":"external_account"}"#).unwrap();
    let said = open(&address("gs://shop-runs"), &[("GOOGLE_APPLICATION_CREDENTIALS", file.to_str().unwrap())], &home).err().expect("refused");
    assert!(matches!(said, BucketError::Unsigned(ref why) if why.contains("external_account")), "{said:?}");
}
