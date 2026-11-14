//! Deploying a project's store against a Cloudflare double: the tunnel's sign-in is tried first, a
//! refusal of every sign-in asks for a pasted token and changes nothing, a pasted token is kept once
//! it works, and a refused rate limit puts the Worker up without it.
//! `docs/contracts/central-store.md`, *Deploying a store*.

use std::sync::Arc;

use adapter_traits::{Cloudflare, CloudflareTokens, Secret};
use testkit::{CloudflareCall, FakeCloudflare, FakeCloudflareTokens, FakeWorkProduct};

use crate::store_deploy::{Deployed, Deploying, worker_name};
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

const ACCOUNT: &str = "acc0001";
const TUNNEL: &str = "tunnel-token";
const PASTED: &str = "pasted-token";
const PAGE: &str = "https://dash.example/tokens";

fn deploying(cloudflare: &Arc<FakeCloudflare>, tokens: &Arc<FakeCloudflareTokens>, tunnel: bool) -> Deploying {
    Deploying {
        cloudflare: Arc::clone(cloudflare) as Arc<dyn Cloudflare>,
        tokens: Arc::clone(tokens) as Arc<dyn CloudflareTokens>,
        tunnel: Arc::new(move || tunnel.then(|| (ACCOUNT.to_string(), Secret::new(TUNNEL.to_string())))),
        script: Arc::from("export default {};"),
        token_page: PAGE.to_string(),
    }
}

fn uploaded_with(cloudflare: &FakeCloudflare) -> Vec<String> {
    cloudflare.calls().into_iter().filter(|call: &CloudflareCall| call.method == "upload_worker").map(|call| call.token).collect()
}

#[test]
fn a_deploy_uses_the_tunnels_sign_in_and_keeps_no_token() {
    let cloudflare = Arc::new(FakeCloudflare::new().accepts(TUNNEL, &[ACCOUNT]));
    let tokens = Arc::new(FakeCloudflareTokens::new());
    let name = worker_name(None);

    let deployed = deploying(&cloudflare, &tokens, true).deploy(&name, None, None).expect("answered");

    let Deployed::Up(up) = deployed else { panic!("the tunnel's sign-in deploys: {deployed:?}") };
    assert_eq!(up.address, "https://armada-store.person.workers.dev");
    assert!(up.limited, "joins are rate limited where Cloudflare takes the limit");
    assert_eq!(uploaded_with(&cloudflare), vec![TUNNEL.to_string()]);
    assert_eq!(cloudflare.workers(), vec![(ACCOUNT.to_string(), "armada-store".to_string())]);
    assert_eq!(tokens.kept(ACCOUNT), None, "the tunnel's token is never kept");
}

#[test]
fn a_refused_tunnel_sign_in_asks_for_a_token_and_changes_nothing() {
    let cloudflare = Arc::new(FakeCloudflare::new().refuses_at(TUNNEL, &[ACCOUNT], &["upload_worker", "workers_dev"]));
    let tokens = Arc::new(FakeCloudflareTokens::new());

    let deployed = deploying(&cloudflare, &tokens, true).deploy(&worker_name(None), None, None).expect("answered");

    assert!(matches!(deployed, Deployed::NeedsToken(_)), "{deployed:?}");
    assert_eq!(uploaded_with(&cloudflare), vec![TUNNEL.to_string()], "the tunnel's sign-in was tried first");
    assert!(cloudflare.workers().is_empty(), "nothing went up");
    assert_eq!(tokens.kept(ACCOUNT), None);
}

#[test]
fn a_mac_with_no_tunnel_asks_for_a_token_without_calling_cloudflare() {
    let cloudflare = Arc::new(FakeCloudflare::new());
    let tokens = Arc::new(FakeCloudflareTokens::new());

    let deployed = deploying(&cloudflare, &tokens, false).deploy(&worker_name(None), None, None).expect("answered");

    assert!(matches!(deployed, Deployed::NeedsToken(_)), "{deployed:?}");
    assert!(cloudflare.calls().is_empty());
}

#[test]
fn a_pasted_token_is_the_only_one_tried_and_is_kept_once_it_works() {
    let cloudflare = Arc::new(FakeCloudflare::new().refuses(TUNNEL).accepts(PASTED, &["other01", ACCOUNT]));
    let tokens = Arc::new(FakeCloudflareTokens::new());
    let deploying = deploying(&cloudflare, &tokens, true);

    let deployed = deploying.deploy(&worker_name(None), None, Some(Secret::new(PASTED.to_string()))).expect("answered");

    assert!(matches!(deployed, Deployed::Up(_)), "{deployed:?}");
    assert!(cloudflare.tokens_used().iter().all(|token| token == PASTED), "{:?}", cloudflare.tokens_used());
    assert_eq!(cloudflare.workers(), vec![(ACCOUNT.to_string(), "armada-store".to_string())], "the tunnel's account where the token reaches it");
    assert_eq!(tokens.kept(ACCOUNT).as_deref(), Some(PASTED));

    // The next deploy reads the kept token before the tunnel's.
    deploying.deploy(&worker_name(None), None, None).expect("answered");
    assert_eq!(uploaded_with(&cloudflare), vec![PASTED.to_string(), PASTED.to_string()]);
}

#[test]
fn a_pasted_token_cloudflare_refuses_is_not_kept() {
    let cloudflare = Arc::new(FakeCloudflare::new());
    let tokens = Arc::new(FakeCloudflareTokens::new());

    let deployed = deploying(&cloudflare, &tokens, true).deploy(&worker_name(None), None, Some(Secret::new(PASTED.to_string()))).expect("answered");

    assert!(matches!(deployed, Deployed::NeedsToken(_)), "{deployed:?}");
    assert_eq!(tokens.kept(ACCOUNT), None);
}

#[test]
fn a_refused_rate_limit_puts_the_worker_up_without_it() {
    let cloudflare = Arc::new(FakeCloudflare::new().accepts(TUNNEL, &[ACCOUNT]).refuses_the_limit());
    let tokens = Arc::new(FakeCloudflareTokens::new());
    let team = core_model::StoreTeam::new("Mele-Labs/core").expect("a team");

    let deployed = deploying(&cloudflare, &tokens, true).deploy(&worker_name(Some(&team)), Some(&team), None).expect("answered");

    let Deployed::Up(up) = deployed else { panic!("{deployed:?}") };
    assert!(!up.limited);
    let uploads = cloudflare.uploads();
    assert_eq!(uploads.iter().map(|upload| upload.enrol_limit).collect::<Vec<_>>(), vec![true, false]);
    assert!(uploads.iter().all(|upload| upload.name == "armada-store-mele-labs-core" && upload.team.as_deref() == Some("mele-labs/core")));
}

/// Through Fleet: a refusal of the tunnel's sign-in names no store and answers the token page; the
/// token pasted from it deploys, and the project's file names the store with its Worker.
#[tokio::test]
async fn a_deploy_names_the_store_in_its_project_and_a_refusal_names_none() {
    let home = TempDir::new();
    let cloudflare = Arc::new(FakeCloudflare::new().refuses(TUNNEL).accepts(PASTED, &[ACCOUNT]));
    let tokens = Arc::new(FakeCloudflareTokens::new());
    let mut fitted = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fitted.deploying = deploying(&cloudflare, &tokens, true);
    let fleet = crate::daemon::Fleet::assembled(fitted);
    let project = crate::tests::red_claim::a_project_of_every_repository(&fleet).await;

    let refused = api::Projects::deploy_project_store(&fleet, project.clone(), ipc::DeployProjectStore::default()).await.expect("answered");
    assert!(matches!(&refused, ipc::StoreDeployed::NeedsToken { page, .. } if page == PAGE), "{refused:?}");
    let state = api::Projects::get_project_store(&fleet, project.clone()).await.expect("read");
    assert_eq!(state.store, None, "a refusal names no store");
    assert_eq!(state.deploy, ipc::StoreDeploy::TryTunnel);

    let pasted = ipc::DeployProjectStore { team: None, cloudflare_token: Some(format!(" {PASTED} ")) };
    let ipc::StoreDeployed::Deployed { state, join_said } =
        api::Projects::deploy_project_store(&fleet, project.clone(), pasted).await.expect("answered")
    else {
        panic!("the pasted token deploys");
    };
    let store = state.store.expect("the project names its store");
    assert_eq!(store.address, "https://armada-store.person.workers.dev");
    assert_eq!(store.worker.as_deref(), Some("armada-store"));
    assert!(store.deployed.is_some());
    assert!(!store.joins_unlimited);
    assert_eq!(state.deploy, ipc::StoreDeploy::Ready, "the pasted token is kept for the next deploy");
    assert!(join_said.is_some(), "this Mac has no profile to join as");
    assert_eq!(state.join, ipc::StoreJoin::NoProfile);
    let wire = ipc::encode(&state).expect("encodes");
    assert!(!wire.contains(PASTED), "the token is never answered back");
}
