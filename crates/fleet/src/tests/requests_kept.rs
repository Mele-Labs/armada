//! Requests' two files for a repository: the person's own at the repository's place, saved and
//! read back, in a folder or a bucket; the repository's committed `.armada/requests.toml` beside
//! it, the person's own winning; a file that will not read never saved over; each secret's value in
//! the Keychain and never in a file; and an agent's save into a collection.

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use api::{Commands, Refusal, Requests};
use ipc::{
    DeleteRequestEnvironment, Placed, RequestCollection, RequestEnvironmentWrite, RequestPlace, RequestToSave, RunCloud,
    RunMemoryPlace, RunPlace, SaveRequestCollection, SaveRequestEnvironment, SaveRequestToCollection,
    SavedRequest, SessionRow, VariableRead, VariableWrite,
};
use testkit::FakeBucket;

use super::red_claim::Planted;
use super::requesting::Held;
use super::session_host::{rig_fitted, Rig};
use crate::run_memory::places;

const TOKEN: &str = "t0ken-0f-the-0wner-1234";

/// A rig whose Keychain the test holds.
fn rig_holding() -> (Rig, Arc<Held>) {
    let held = Arc::new(Held::default());
    let planted = Arc::clone(&held);
    let secrets: Arc<dyn adapter_traits::RequestSecrets + Send + Sync> = planted;
    (rig_fitted(move |fittings| fittings.requests.secrets = secrets), held)
}

fn own_file(rig: &Rig) -> PathBuf {
    places::file_for(&PathBuf::from(&rig.fleet.host().kit_home), "requests", &rig.root)
}

fn collection(id: &str, name: &str) -> RequestCollection {
    RequestCollection {
        id: id.to_string(),
        name: name.to_string(),
        headers: Vec::new(),
        requests: vec![SavedRequest {
            id: "me".to_string(),
            name: "Me".to_string(),
            method: "GET".to_string(),
            url: "{{base}}/v1/me".to_string(),
            headers: Vec::new(),
            body: None,
        }],
    }
}

fn saving(rig: &Rig, collection: RequestCollection) -> SaveRequestCollection {
    SaveRequestCollection { repository: Some(rig.root.clone()), session_id: None, collection }
}

fn environment(rig: &Rig, variables: Vec<VariableWrite>) -> SaveRequestEnvironment {
    SaveRequestEnvironment {
        repository: Some(rig.root.clone()),
        session_id: None,
        environment: RequestEnvironmentWrite { name: "staging".to_string(), variables },
    }
}

fn secret(name: &str, value: Option<&str>) -> VariableWrite {
    VariableWrite::Secret { name: name.to_string(), value: value.map(str::to_string) }
}

fn refused(refusal: Refusal) -> (u16, String) {
    let status = refusal.status();
    match refusal {
        Refusal::NotFound(error) | Refusal::IllegalMove(error) | Refusal::Unacceptable(error) | Refusal::Fault(error) => {
            (status, error.code)
        }
    }
}

#[tokio::test]
async fn a_collection_saved_is_read_back_from_the_persons_own_file_at_the_repositorys_place() {
    let (rig, _) = rig_holding();
    let empty = rig.fleet.get_requests(None, Some(rig.root.clone()), None).await.expect("read");
    assert_eq!((empty.place, empty.collections.len(), empty.not_read.len()), (RunPlace::ThisMac, 0, 0));
    let read = rig.fleet.save_request_collection(None, saving(&rig, collection("users", "Users"))).await.expect("saved");
    assert_eq!(read.collections, vec![Placed { item: collection("users", "Users"), place: RequestPlace::Yours, hides: false }]);
    let text = std::fs::read_to_string(own_file(&rig)).expect("written in Kit's folder");
    assert!(text.contains("[[collection]]") && text.contains("[[collection.requests]]"), "{text}");
    assert_eq!(rig.fleet.get_requests(None, Some(rig.root.clone()), None).await.expect("read"), read);

    let mut broken = collection("users", "Users");
    broken.requests[0].method = "TRACE".to_string();
    let (status, code) = refused(rig.fleet.save_request_collection(None, saving(&rig, broken)).await.expect_err("refused"));
    assert_eq!((status, code.as_str()), (422, "fleet.request_collection_refused"));
    let (status, code) = refused(rig.fleet.get_requests(None, None, None).await.expect_err("unplaced"));
    assert_eq!((status, code.as_str()), (422, "fleet.requests_unplaced"));
}

#[tokio::test]
async fn a_file_that_will_not_read_is_named_and_never_saved_over() {
    let (rig, _) = rig_holding();
    std::fs::create_dir_all(own_file(&rig).parent().unwrap()).unwrap();
    std::fs::write(own_file(&rig), "version = 1\n[[collection]]\nid = \"\"\n").unwrap();
    let read = rig.fleet.get_requests(None, Some(rig.root.clone()), None).await.expect("read");
    assert_eq!(read.not_read.len(), 1, "{read:?}");
    assert_eq!(read.not_read[0].place, RequestPlace::Yours);
    let (status, code) = refused(rig.fleet.save_request_collection(None, saving(&rig, collection("users", "Users"))).await.expect_err("refused"));
    assert_eq!((status, code.as_str()), (409, "fleet.requests_unread"));
    assert!(std::fs::read_to_string(own_file(&rig)).unwrap().contains("id = \"\""), "left as the person broke it");
}

#[tokio::test]
async fn a_bucket_keeps_the_persons_own_file_as_an_object() {
    let bucket = Arc::new(FakeBucket::new());
    let held = Arc::clone(&bucket);
    let rig = rig_fitted(move |fittings| fittings.reaching = Arc::new(Planted::with_bucket(held)));
    let place = RunMemoryPlace {
        repository: rig.root.clone(),
        place: RunPlace::Cloud,
        synced_folder: None,
        cloud: Some(RunCloud::Bucket("s3://shop-runs/armada".into())),
    };
    rig.fleet.set_run_memory_place(place).await.expect("chosen");
    let read = rig.fleet.save_request_collection(None, saving(&rig, collection("users", "Users"))).await.expect("saved");
    assert_eq!((read.place, read.collections.len()), (RunPlace::Cloud, 1));
    let object = places::object_for("requests", &rig.root);
    let text = String::from_utf8(bucket.object(&object).expect("an object")).unwrap();
    assert!(text.contains("id = \"users\""), "{text}");
    assert!(!own_file(&rig).exists(), "not in Kit's folder");
}

#[tokio::test]
async fn the_repositorys_committed_file_is_listed_beside_and_the_persons_own_hides_it() {
    let (rig, _) = rig_holding();
    let committed = "version = 1\n\n[[collection]]\nid = \"users\"\nname = \"Users, committed\"\n\n[[collection]]\nid = \"orders\"\nname = \"Orders\"\n\n[[environment]]\nname = \"staging\"\nvariables = [{ name = \"base\", value = \"https://staging.example.test\" }]\n";
    std::fs::create_dir_all(PathBuf::from(&rig.root).join(".armada")).unwrap();
    std::fs::write(PathBuf::from(&rig.root).join(".armada/requests.toml"), committed).unwrap();
    let read = rig.fleet.save_request_collection(None, saving(&rig, collection("users", "Users"))).await.expect("saved");
    let listed: Vec<(&str, &str, RequestPlace, bool)> = read
        .collections
        .iter()
        .map(|one| (one.item.id.as_str(), one.item.name.as_str(), one.place, one.hides))
        .collect();
    assert_eq!(listed, vec![("users", "Users", RequestPlace::Yours, true), ("orders", "Orders", RequestPlace::Repository, false)]);
    assert_eq!((read.environments[0].item.name.as_str(), read.environments[0].place), ("staging", RequestPlace::Repository));

    std::fs::write(PathBuf::from(&rig.root).join(".armada/requests.toml"), "version = 1\n[[environment]]\nname = \"x\"\nvariables = [{ name = \"t\", secret = true, value = \"leaked\" }]\n").unwrap();
    let read = rig.fleet.get_requests(None, Some(rig.root.clone()), None).await.expect("read");
    assert_eq!(read.not_read.iter().map(|one| one.place).collect::<Vec<_>>(), vec![RequestPlace::Repository]);
    assert!(!read.not_read[0].why.contains("leaked"), "{}", read.not_read[0].why);
}

#[tokio::test]
async fn a_secret_is_kept_in_the_keychain_and_never_in_the_file() {
    let (rig, held) = rig_holding();
    let plain = VariableWrite::Plain { name: "base".to_string(), value: "https://staging.example.test".to_string() };
    let read = rig.fleet.save_request_environment(None, environment(&rig, vec![plain.clone(), secret("token", Some(TOKEN))])).await.expect("saved");
    let account = format!("{}/staging/token", places::key_of(&rig.root));
    assert_eq!(held.accounts(), vec![account.clone()]);
    assert_eq!(read.environments[0].item.variables[1], VariableRead::Secret { name: "token".to_string(), set: true });
    let text = std::fs::read_to_string(own_file(&rig)).unwrap();
    assert!(text.contains("secret = true") && !text.contains(TOKEN), "{text}");

    // Sent with no value, the kept one stays.
    rig.fleet.save_request_environment(None, environment(&rig, vec![secret("token", None)])).await.expect("saved");
    assert_eq!(held.values.lock().unwrap().get(&account).map(String::as_str), Some(TOKEN));
    // Sent as plain, its item goes; left out, the same.
    rig.fleet.save_request_environment(None, environment(&rig, vec![secret("token", Some(TOKEN)), secret("other", Some("0ther-value"))])).await.expect("saved");
    let as_plain = VariableWrite::Plain { name: "token".to_string(), value: "public".to_string() };
    rig.fleet.save_request_environment(None, environment(&rig, vec![as_plain])).await.expect("saved");
    assert!(held.accounts().is_empty(), "{:?}", held.accounts());

    let (status, code) = refused(rig.fleet.save_request_environment(None, environment(&rig, vec![secret("bad name", Some(TOKEN))])).await.expect_err("refused"));
    assert_eq!((status, code.as_str()), (422, "fleet.request_environment_refused"));
    held.refusing.store(true, Ordering::SeqCst);
    let before = std::fs::read_to_string(own_file(&rig)).unwrap();
    let (status, code) = refused(rig.fleet.save_request_environment(None, environment(&rig, vec![secret("token", Some(TOKEN))])).await.expect_err("refused"));
    assert_eq!((status, code.as_str()), (500, "fleet.request_secret_unkept"));
    assert_eq!(std::fs::read_to_string(own_file(&rig)).unwrap(), before, "the file is not written");
}

fn named_environment(rig: &Rig, name: &str, variables: Vec<VariableWrite>) -> SaveRequestEnvironment {
    let mut save = environment(rig, variables);
    save.environment.name = name.to_string();
    save
}

fn deleting(rig: &Rig, name: &str) -> DeleteRequestEnvironment {
    DeleteRequestEnvironment { repository: Some(rig.root.clone()), session_id: None, name: name.to_string() }
}

fn base() -> VariableWrite {
    VariableWrite::Plain { name: "base".to_string(), value: "https://staging.example.test".to_string() }
}

/// Pins what a save already does: a secret left out goes from the list and from the Keychain.
#[tokio::test]
async fn a_secret_left_out_of_a_save_leaves_the_list_and_the_keychain() {
    let (rig, held) = rig_holding();
    rig.fleet.save_request_environment(None, environment(&rig, vec![base(), secret("token", Some(TOKEN))])).await.expect("saved");
    assert_eq!(held.accounts().len(), 1);
    rig.fleet.save_request_environment(None, environment(&rig, vec![base()])).await.expect("saved");
    let read = rig.fleet.get_requests(None, Some(rig.root.clone()), None).await.expect("read");
    let staging = read.environments.iter().find(|one| one.item.name == "staging").expect("staging listed");
    let names: Vec<&str> = staging
        .item
        .variables
        .iter()
        .map(|one| match one {
            VariableRead::Plain { name, .. } | VariableRead::Secret { name, .. } => name.as_str(),
        })
        .collect();
    assert_eq!(names, vec!["base"]);
    assert!(held.accounts().is_empty(), "{:?}", held.accounts());
}

#[tokio::test]
async fn deleting_an_environment_forgets_its_secrets_and_leaves_the_list() {
    let (rig, held) = rig_holding();
    let staging = vec![base(), secret("token", Some(TOKEN)), secret("cookie", Some("c00kie-value"))];
    rig.fleet.save_request_environment(None, named_environment(&rig, "staging", staging)).await.expect("saved");
    rig.fleet.save_request_environment(None, named_environment(&rig, "test", vec![base(), secret("token", Some(TOKEN))])).await.expect("saved");
    let kept_for_test = format!("{}/test/token", places::key_of(&rig.root));
    assert_eq!(held.accounts().len(), 3, "{:?}", held.accounts());

    let read = rig.fleet.delete_request_environment(None, deleting(&rig, "staging")).await.expect("deleted");
    let listed: Vec<&str> = read.environments.iter().map(|one| one.item.name.as_str()).collect();
    assert_eq!(listed, vec!["test"]);
    assert_eq!(held.accounts(), vec![kept_for_test]);
    let test = &read.environments[0].item.variables;
    assert_eq!(test[1], VariableRead::Secret { name: "token".to_string(), set: true });
    let again = rig.fleet.get_requests(None, Some(rig.root.clone()), None).await.expect("read");
    assert_eq!(again, read);
    let text = std::fs::read_to_string(own_file(&rig)).unwrap();
    assert!(!text.contains("name = \"staging\"") && text.contains("name = \"test\""), "{text}");
}

#[tokio::test]
async fn a_delete_is_refused_for_an_unknown_name_an_unread_file_and_a_refusing_keychain() {
    let (rig, held) = rig_holding();
    rig.fleet.save_request_environment(None, environment(&rig, vec![base(), secret("token", Some(TOKEN))])).await.expect("saved");
    let (status, code) = refused(rig.fleet.delete_request_environment(None, deleting(&rig, "nowhere")).await.expect_err("refused"));
    assert_eq!((status, code.as_str()), (404, "fleet.no_such_request_environment"));

    // The repository's is not the person's to delete.
    let committed = "version = 1\n[[environment]]\nname = \"shared\"\nvariables = [{ name = \"base\", value = \"https://shared.example.test\" }]\n";
    std::fs::create_dir_all(PathBuf::from(&rig.root).join(".armada")).unwrap();
    std::fs::write(PathBuf::from(&rig.root).join(".armada/requests.toml"), committed).unwrap();
    let (status, code) = refused(rig.fleet.delete_request_environment(None, deleting(&rig, "shared")).await.expect_err("refused"));
    assert_eq!((status, code.as_str()), (404, "fleet.no_such_request_environment"));

    held.refusing.store(true, Ordering::SeqCst);
    let before = std::fs::read_to_string(own_file(&rig)).unwrap();
    let (status, code) = refused(rig.fleet.delete_request_environment(None, deleting(&rig, "staging")).await.expect_err("refused"));
    assert_eq!((status, code.as_str()), (500, "fleet.request_secret_unkept"));
    assert_eq!(std::fs::read_to_string(own_file(&rig)).unwrap(), before, "the file is not written");
    held.refusing.store(false, Ordering::SeqCst);

    std::fs::write(own_file(&rig), "version = 1\n[[collection]]\nid = \"\"\n").unwrap();
    let (status, code) = refused(rig.fleet.delete_request_environment(None, deleting(&rig, "staging")).await.expect_err("refused"));
    assert_eq!((status, code.as_str()), (409, "fleet.requests_unread"));
    assert!(std::fs::read_to_string(own_file(&rig)).unwrap().contains("id = \"\""), "left as the person broke it");
}

fn to_save(name: &str, url: &str) -> RequestToSave {
    RequestToSave { name: name.to_string(), method: "GET".to_string(), url: url.to_string(), headers: Vec::new(), body: None }
}

#[tokio::test]
async fn an_agents_save_makes_or_copies_a_collection_replaces_by_name_and_writes_a_saved_row() {
    let (rig, _) = rig_holding();
    let id = rig.start().await;
    // Read from the Job's own tree where it holds a slot, and the checkout's otherwise.
    let tree = rig.fleet.job_tree_in(&rig.root, Some(&id)).await.map_or_else(|| rig.root.clone(), |tree| tree.path);
    let committed = "version = 1\n[[collection]]\nid = \"orders\"\nname = \"Orders\"\nheaders = [{ name = \"X-Team\", value = \"shop\" }]\n";
    std::fs::create_dir_all(PathBuf::from(&tree).join(".armada")).unwrap();
    std::fs::write(PathBuf::from(&tree).join(".armada/requests.toml"), committed).unwrap();
    let save = |collection: &str, request: RequestToSave| SaveRequestToCollection {
        session_id: Some(id.clone()),
        collection: collection.to_string(),
        request,
    };
    let saved = rig.fleet.save_request_to_collection(None, save("Health checks", to_save("Who am I", "{{base}}/v1/me"))).await.expect("saved");
    assert_eq!((saved.collection_id.as_str(), saved.request_id.as_str()), ("health-checks", "who-am-i"));
    let again = rig.fleet.save_request_to_collection(None, save("health-checks", to_save("Who am I", "{{base}}/v2/me"))).await.expect("saved");
    assert_eq!(again, saved, "a request of that name is replaced and keeps its id");
    let other = rig.fleet.save_request_to_collection(None, save("Health checks", to_save("who am i", "{{base}}/v3"))).await.expect("saved");
    assert_eq!(other.request_id, "who-am-i-2", "a new name whose id is taken");

    let copied = rig.fleet.save_request_to_collection(None, save("Orders", to_save("List", "{{base}}/orders"))).await.expect("saved");
    assert_eq!(copied.collection_id, "orders");
    let read = rig.fleet.get_requests(None, Some(rig.root.clone()), None).await.expect("read");
    let orders = read.collections.iter().find(|one| one.item.id == "orders").expect("listed");
    assert_eq!(orders.place, RequestPlace::Yours, "copied into the person's own");
    assert_eq!(orders.item.headers[0].name, "X-Team", "the repository's, copied whole");
    let health = read.collections.iter().find(|one| one.item.id == "health-checks").expect("listed");
    assert_eq!(health.item.requests[0].url, "{{base}}/v2/me");
    assert!(!std::fs::read_to_string(PathBuf::from(&tree).join(".armada/requests.toml")).unwrap().contains("List"));

    let rows = rig.rows(&id).await;
    let saved_rows: Vec<(&str, &str, &str)> = rows
        .iter()
        .filter_map(|row| match row {
            SessionRow::Saved { name, collection_id, request, .. } => Some((name.as_str(), collection_id.as_str(), request.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(saved_rows.len(), 4, "{rows:?}");
    assert_eq!(saved_rows[3], ("List", "orders", "list"));
    let (status, code) = refused(
        rig.fleet
            .save_request_to_collection(None, SaveRequestToCollection { session_id: None, collection: "x".into(), request: to_save("a", "b") })
            .await
            .expect_err("unplaced"),
    );
    assert_eq!((status, code.as_str()), (422, "fleet.requests_unplaced"));
}
