//! The system map of two projects, for a walk, a preview and the tests with no Fleet: armada, and a small one with a web page, a
//! browser extension and a script (auto-ynab). The first project the mock board holds takes the first, any other the second.

use ipc::{FeatureJob, PartIssue, PartOrigin, PartOwner, PartPull, PartWatch, PathStop, SessionId, SystemFeature, SystemFolder, SystemLink, SystemMap, SystemPart, SystemPartAnswers, SystemPartKind};

use SystemPartKind::{Entry, Outside, Person, Piece, Service, Store};

fn armada(project: &str) -> bool {
    project == crate::features::mock::PROJECT
}

fn detected(from: &str) -> PartOrigin {
    PartOrigin::Detected { from: from.into() }
}

fn part(id: &str, name: &str, kind: SystemPartKind, summary: &str, paths: &[&str], zooms: bool) -> SystemPart {
    SystemPart {
        id: id.into(),
        name: name.into(),
        kind,
        summary: summary.into(),
        within: None,
        manifest_id: None,
        paths: paths.iter().map(|path| path.to_string()).collect(),
        zooms,
        origin: detected(if paths.is_empty() { "ci" } else { "cargo" }),
    }
}

fn piece(id: &str, name: &str, within: &str, kind: SystemPartKind, summary: &str, paths: &[&str]) -> SystemPart {
    SystemPart { within: Some(within.into()), ..part(id, name, kind, summary, paths, false) }
}

fn link(from: &str, to: &str, label: &str) -> SystemLink {
    SystemLink { from: from.into(), to: to.into(), label: Some(label).filter(|label| !label.is_empty()).map(str::to_string), step: None }
}

fn step(from: &str, to: &str, label: &str, step: u32) -> SystemLink {
    SystemLink { step: Some(step), ..link(from, to, label) }
}

fn feature(slug: &str, title: &str, parts: &[&str]) -> SystemFeature {
    SystemFeature { slug: slug.into(), title: title.into(), parts: parts.iter().map(|part| part.to_string()).collect() }
}

fn stop(step: u32, part: &str, files: &[&str]) -> PathStop {
    PathStop { step, part: part.into(), files: files.iter().map(|file| file.to_string()).collect() }
}

fn empty(project: &str) -> SystemMap {
    SystemMap { project: project.into(), part: None, feature: None, parts: Vec::new(), links: Vec::new(), features: Vec::new(), stops: Vec::new(), librarian_at: None }
}

/// One level of the map: level one with neither argument, level two with a part, level three with a feature.
pub fn map(project: &str, part: Option<&str>, feature: Option<&str>) -> SystemMap {
    let base = SystemMap { part: part.map(str::to_string), feature: feature.map(str::to_string), ..empty(project) };
    match (armada(project), part, feature) {
        (true, None, _) => armada_one(base),
        (true, Some("bridge"), None) => armada_two(base),
        (true, Some("bridge"), Some(_)) => armada_three(base),
        (false, None, _) => ynab_one(base),
        (false, Some("web"), None) => ynab_two(base),
        (false, Some("web"), Some(_)) => ynab_three(base),
        _ => base,
    }
}

fn armada_one(base: SystemMap) -> SystemMap {
    let parts = vec![
        part("you", "You", Person, "The owner. From git authors and Bridge sign-in.", &[], false),
        part("agents", "Agents", Person, "Claude Code sessions Fleet runs as Jobs.", &[], false),
        part("bridge", "Bridge", Entry, "The native app. Talks to Fleet over its wire, nothing else.", &["crates/bridge-native/"], true),
        part("classic", "Bridge Classic", Entry, "The Electron app.", &["apps/desktop/"], false),
        part("pocket", "Pocket", Entry, "The phone page.", &["apps/pocket/"], false),
        part("cli", "armada CLI", Entry, "The command.", &["crates/armada/"], false),
        part("api", "Fleet API", Entry, "What agents call: HTTP and the MCP tools, served by Fleet.", &["crates/ipc/operations/", "crates/fleet/src/session_host/"], false),
        part("fleet", "Fleet daemon", Service, "Runs Jobs, checks their work, serves every entry point.", &["crates/fleet/"], false),
        part("central", "Central store", Store, "Claims and profiles across Macs.", &["apps/store/"], false),
        part("sqlite", "armada.sqlite", Store, "Jobs, features, issue lists.", &["crates/store/"], false),
        part("gh", "GitHub", Outside, "Pull requests, issues, checks.", &[], false),
        part("cc", "Claude Code", Outside, "The agent CLI each Job runs.", &[], false),
        part("cf", "Cloudflare", Outside, "The tunnel Pocket reaches Fleet through.", &[], false),
    ];
    let links = vec![
        link("you", "bridge", ""),
        link("you", "classic", ""),
        link("you", "pocket", ""),
        link("you", "cli", ""),
        link("agents", "api", ""),
        link("bridge", "fleet", "wire"),
        link("classic", "fleet", "wire"),
        link("pocket", "fleet", "HTTPS"),
        link("cli", "fleet", "HTTP"),
        link("api", "fleet", "served by"),
        link("fleet", "sqlite", "reads, writes"),
        link("fleet", "central", "claims"),
        link("fleet", "gh", "PRs"),
        link("fleet", "cc", "runs"),
        link("fleet", "cf", "tunnel"),
    ];
    let features = vec![
        feature("dashboard", "Dashboard", &["bridge"]),
        feature("jobs", "Jobs", &["bridge", "fleet", "api"]),
        feature("features", "Features", &["bridge", "fleet"]),
        feature("reviews", "Reviews", &["bridge", "fleet"]),
        feature("settings", "Settings", &["bridge"]),
        feature("pocket", "Pocket", &["pocket"]),
        feature("librarian", "Librarian", &["fleet", "api"]),
        feature("watches", "Watches", &["fleet", "api"]),
    ];
    SystemMap { parts, links, features, ..base }
}

fn armada_two(base: SystemMap) -> SystemMap {
    let bridge = |id: &str, name: &str, summary: &str, path: &str| piece(id, name, "bridge", Piece, summary, &[path]);
    let fleet = |id: &str, name: &str, summary: &str, path: &str| piece(id, name, "fleet", Piece, summary, &[path]);
    let parts = vec![
        bridge("b_board", "project_board", "Draws the Dashboard and its blocks.", "crates/bridge-native/src/project_board/"),
        bridge("b_feat", "features", "The Project place: Overview, catalog, floor plans.", "crates/bridge-native/src/features/"),
        bridge("b_jobs", "jobs", "The Jobs list and a Job's view.", "crates/bridge-native/src/jobs/"),
        bridge("b_rev", "reviews", "Pull requests to review.", "crates/bridge-native/src/reviews/"),
        bridge("b_set", "settings", "Settings and project values.", "crates/bridge-native/src/settings/"),
        fleet("f_board", "project_board", "Reads and saves a project's board.", "crates/fleet/src/project_board.rs"),
        fleet("f_lib", "feature_library", "Catalog, librarian, ranking.", "crates/fleet/src/feature_library/"),
        fleet("f_daemon", "daemon", "Jobs, pull requests, the event stream.", "crates/fleet/src/daemon.rs"),
        fleet("f_vals", "values", "A project's values file.", "crates/fleet/src/values.rs"),
        part("s_sql", "armada.sqlite", Store, "Jobs, features, issue lists.", &["crates/store/"], false),
        part("s_vals", "Project values file", Store, "One per project.", &[".armada/values.toml"], false),
    ];
    let links = vec![
        link("b_board", "f_board", "get_project_board"),
        link("b_board", "f_lib", "list_ranked_issues"),
        link("b_feat", "f_lib", "get_feature_page"),
        link("b_jobs", "f_daemon", "list_jobs"),
        link("b_rev", "f_daemon", "get_pull_request"),
        link("b_set", "f_vals", "edit_project"),
        link("f_board", "s_vals", "look.board"),
        link("f_lib", "s_sql", ""),
        link("f_daemon", "s_sql", ""),
        link("f_vals", "s_vals", ""),
    ];
    let features = vec![
        feature("dashboard", "Dashboard", &["b_board"]),
        feature("feature-catalog", "Feature catalog", &["b_feat"]),
        feature("jobs-list", "Jobs list", &["b_jobs"]),
        feature("reviews", "Reviews", &["b_rev"]),
        feature("settings", "Settings", &["b_set"]),
    ];
    SystemMap { parts, links, features, ..base }
}

fn armada_three(base: SystemMap) -> SystemMap {
    let parts = armada_two(empty("armada")).parts.into_iter().filter(|one| ["b_board", "f_board", "f_lib", "s_vals", "s_sql"].contains(&one.id.as_str())).collect();
    let links = vec![
        step("b_board", "f_board", "get and save_project_board", 1),
        step("f_board", "s_vals", "writes", 2),
        step("b_board", "f_lib", "list_ranked_issues", 3),
        step("f_lib", "s_sql", "reads", 4),
    ];
    let stops = vec![
        stop(1, "b_board", &["view.rs", "layout.rs", "presets.rs", "wire.rs"]),
        stop(2, "f_board", &["project_board.rs", "values.rs"]),
        stop(2, "s_vals", &["config/src/values.rs"]),
        stop(3, "f_lib", &["ranking.rs"]),
        stop(4, "s_sql", &["store/src/features.rs"]),
    ];
    SystemMap { parts, links, stops, features: Vec::new(), ..base }
}

fn ynab_one(base: SystemMap) -> SystemMap {
    let mut bank = part("bank", "Bank exports", Outside, "The files you download from your bank.", &[], false);
    bank.origin = PartOrigin::Librarian { addition: "bank-exports".into(), why: "ynab_import.py reads OFX and CSV exports from ./inbox, which you download from your bank.".into() };
    let parts = vec![
        part("you", "You", Person, "Runs it on this Mac.", &[], false),
        part("web", "Web app", Entry, "React screens that talk only to the local server.", &["frontend/"], true),
        part("grab", "Order Grabber", Entry, "Sends an order's items from a retailer page.", &["extension/"], false),
        part("cli", "ynab_import.py", Entry, "Batch import without the app.", &["ynab_import.py"], false),
        part("api", "FastAPI backend", Service, "Every call to YNAB goes through it.", &["backend/app/"], false),
        part("sess", "Session files", Store, "One per session, no database.", &["backend/sessions/"], false),
        part("ynab", "YNAB API", Outside, "Budgets, categories, transactions.", &[], false),
        part("shops", "Retailer pages", Outside, "Read by the extension's content script.", &[], false),
        bank,
    ];
    let links = vec![
        link("you", "web", ""),
        link("you", "grab", ""),
        link("you", "cli", ""),
        link("web", "api", "localhost"),
        link("grab", "api", "orders/ingest"),
        link("grab", "shops", "reads orders"),
        link("api", "sess", "saves"),
        link("api", "ynab", "HTTPS"),
        link("cli", "bank", "reads ./inbox"),
        link("cli", "ynab", ""),
    ];
    let features = vec![
        feature("connect-budget", "Connect budget", &["web"]),
        feature("upload-reconcile", "Upload and reconcile", &["web", "api"]),
        feature("receipts", "Receipts", &["web", "api"]),
        feature("review-submit", "Review and submit", &["web", "api"]),
        feature("itemized-orders", "Itemized orders", &["grab"]),
        feature("batch-import", "Batch import", &["cli"]),
    ];
    SystemMap { parts, links, features, ..base }
}

fn ynab_two(base: SystemMap) -> SystemMap {
    let screen = |id: &str, name: &str, summary: &str, path: &str| piece(id, name, "web", Piece, summary, &[path]);
    let module = |id: &str, name: &str, summary: &str, path: &str| piece(id, name, "api", Piece, summary, &[path]);
    let parts = vec![
        screen("s_tok", "TokenBudget.jsx", "Paste a token, pick a budget.", "frontend/src/screens/TokenBudget.jsx"),
        screen("s_up", "UploadReconcile.jsx", "Drop bank files per account.", "frontend/src/screens/UploadReconcile.jsx"),
        screen("s_rec", "Receipts.jsx", "Match receipts to transactions.", "frontend/src/screens/Receipts.jsx"),
        screen("s_rev", "Review.jsx", "Line by line, with splits.", "frontend/src/screens/Review.jsx"),
        screen("s_sub", "Submit.jsx", "Writes to YNAB.", "frontend/src/screens/Submit.jsx"),
        module("main", "main.py", "Every route the app calls, under /api.", "backend/app/main.py"),
        module("m_parse", "parsing.py", "OFX, QFX and CSV.", "backend/app/parsing.py"),
        module("m_rcpt", "receipts.py", "PDF receipts and orders.", "backend/app/receipts.py"),
        module("m_rev", "review.py", "Builds and checks review rows.", "backend/app/review.py"),
        module("m_state", "persistence.py", "Session state, saved as JSON.", "backend/app/persistence.py"),
        module("m_ynab", "ynab_client.py", "The only part that calls YNAB.", "backend/app/ynab_client.py"),
        part("o_sess", "Session files", Store, "One per session.", &["backend/sessions/"], false),
        part("o_ynab", "YNAB API", Outside, "api.ynab.com", &[], false),
    ];
    let links = vec![
        link("s_tok", "main", "/api/token"),
        link("s_up", "main", "/accounts/:a/parse"),
        link("s_rec", "main", "/receipts/:store"),
        link("s_rev", "main", "/review"),
        link("s_sub", "main", "/submit"),
        link("main", "m_parse", ""),
        link("main", "m_rcpt", ""),
        link("main", "m_rev", ""),
        link("main", "m_state", ""),
        link("main", "m_ynab", ""),
        link("m_state", "o_sess", ""),
        link("m_ynab", "o_ynab", "HTTPS"),
    ];
    let features = vec![
        feature("connect-budget", "Connect budget", &["s_tok"]),
        feature("upload-reconcile", "Upload and reconcile", &["s_up"]),
        feature("receipts", "Receipts", &["s_rec"]),
        feature("review-submit", "Review and submit", &["s_rev", "s_sub"]),
    ];
    SystemMap { parts, links, features, ..base }
}

fn ynab_three(base: SystemMap) -> SystemMap {
    let parts = ynab_two(empty("auto-ynab")).parts.into_iter().filter(|one| ["s_rev", "s_sub", "main", "m_rev", "m_ynab", "o_sess", "o_ynab"].contains(&one.id.as_str())).collect();
    let links = vec![
        step("s_rev", "main", "GET /review, PUT splits", 1),
        step("main", "m_rev", "checks splits", 2),
        step("s_sub", "main", "POST /submit", 3),
        step("main", "m_ynab", "", 4),
        step("m_ynab", "o_ynab", "HTTPS", 5),
    ];
    let stops = vec![
        stop(1, "s_rev", &["Review.jsx", "SplitEditor.jsx", "api.js"]),
        stop(2, "m_rev", &["review.py", "splits.py"]),
        stop(3, "s_sub", &["Submit.jsx"]),
        stop(4, "main", &["main.py"]),
        stop(5, "m_ynab", &["ynab_client.py"]),
    ];
    SystemMap { parts, links, stops, features: Vec::new(), ..base }
}

fn job(id: &str, title: &str) -> FeatureJob {
    FeatureJob { session_id: SessionId::carried(id), title: title.into() }
}

fn watch(label: &str, kind: &str) -> PartWatch {
    PartWatch { label: label.into(), kind: kind.into() }
}

fn issue(id: &str, title: &str) -> PartIssue {
    PartIssue { id: id.into(), title: title.into() }
}

fn pull(number: u64, title: &str, author: &str) -> PartPull {
    PartPull { number, title: title.into(), author: author.into() }
}

fn blank(project: &str, part: &str, tracker: bool, has_screen: bool) -> SystemPartAnswers {
    SystemPartAnswers {
        project: project.into(),
        part: part.into(),
        feature: None,
        features: Vec::new(),
        code: Vec::new(),
        working: Vec::new(),
        owners: Vec::new(),
        owner_days: 30,
        looks: None,
        has_screen,
        docs: Vec::new(),
        monitor: Vec::new(),
        issues: Vec::new(),
        tracker,
        pulls: Vec::new(),
    }
}

/// The paths the mock gives a part on any of its levels, where the part has its own and nothing else is said of it.
fn paths_of(project: &str, id: &str) -> Vec<String> {
    let found = |map: SystemMap| map.parts.into_iter().find(|part| part.id == id).map(|part| part.paths);
    let all = [map(project, None, None), map(project, Some("bridge"), None), map(project, Some("web"), None)];
    all.into_iter().find_map(found).unwrap_or_default()
}

/// The maps of both levels the mock holds for a project, which what is said of a part is read from, so the panel and the graph
/// agree: the features a part is in are the ones the map lists it under.
fn levels(project: &str) -> Vec<SystemMap> {
    let inside = if armada(project) { "bridge" } else { "web" };
    vec![map(project, None, None), map(project, Some(inside), None)]
}

/// The features whose parts include `id`, on any level.
fn features_of(project: &str, id: &str) -> Vec<SystemFeature> {
    let mut out: Vec<SystemFeature> = Vec::new();
    for feature in levels(project).into_iter().flat_map(|level| level.features) {
        if feature.parts.iter().any(|one| one == id) && !out.iter().any(|held| held.slug == feature.slug) {
            out.push(feature);
        }
    }
    out
}

/// Whether the part draws a screen: an entry point that is one, or a piece of one.
fn has_screen(project: &str, id: &str) -> bool {
    let screens = if armada(project) { ["bridge", "classic", "pocket"].as_slice() } else { ["web"].as_slice() };
    let found = levels(project).into_iter().flat_map(|level| level.parts).find(|part| part.id == id);
    screens.contains(&id) || found.is_some_and(|part| part.within.as_deref().is_some_and(|within| screens.contains(&within)))
}

/// What Fleet knows of one part, by the key the block holds it under: a part's id, a feature as `~feature/slug`, and the whole
/// project as an empty one. A part the mock has nothing for answers with every section empty, as Fleet does for one it has found
/// nothing about.
pub fn answers(project: &str, key: &str) -> SystemPartAnswers {
    let mut answers = match key {
        "" => whole(project),
        feature if feature.starts_with(super::FEATURE_KEY) => of_feature(project, &feature[super::FEATURE_KEY.len()..]),
        part => of_part(project, part),
    };
    match key.strip_prefix(super::FEATURE_KEY) {
        Some(slug) => answers.feature = Some(slug.to_string()),
        None if !key.is_empty() => {
            answers.features = features_of(project, key);
            answers.has_screen = has_screen(project, key);
        }
        None => {}
    }
    answers
}

fn owner(share: u8) -> Vec<PartOwner> {
    vec![PartOwner { name: "Nick Mele".into(), share }]
}

fn armada_issues() -> Vec<PartIssue> {
    vec![issue("armada-2g25", "native-job-open and native-job-tabs walks fail on main"), issue("armada-9x0i", "Live native Reviews hides what Fleet does not serve yet")]
}

fn armada_pulls() -> Vec<PartPull> {
    vec![pull(4023, "Say a stalled step waits on the Job while there is nothing for him to press", "nick"), pull(4022, "Judge types: pick what kind of judge reads it", "nick")]
}

/// The whole project: every feature, the folders its code starts at, and every Job writing in it.
fn whole(project: &str) -> SystemPartAnswers {
    let top = map(project, None, None);
    if armada(project) {
        return SystemPartAnswers {
            features: top.features,
            code: vec!["crates/".into(), "apps/".into(), "docs/".into(), "packages/".into()],
            working: vec![job("job-metrics", "Live metrics blocks"), job("job-librarian", "Librarian on the dashboard"), job("job-reviews", "Redesign Review Sheet"), job("job-judge", "Judge feature update")],
            owners: owner(100),
            has_screen: true,
            docs: vec!["docs/scope.md".into(), "ARCHITECTURE.md".into()],
            monitor: vec![watch("GET :40000/health", "health"), watch("~/Library/Logs/Armada/", "log"), watch("scripts/restart", "run")],
            issues: armada_issues(),
            pulls: armada_pulls(),
            ..blank(project, "", true, true)
        };
    }
    SystemPartAnswers {
        features: top.features,
        code: vec!["frontend/".into(), "backend/".into(), "extension/".into(), "ynab_import.py".into()],
        owners: owner(100),
        has_screen: true,
        docs: vec!["SPEC.md".into()],
        monitor: vec![watch("GET /api/session", "health"), watch("npx vite", "server")],
        ..blank(project, "", false, true)
    }
}

/// One feature: itself, the code of the parts its path runs through, and what is known of it.
fn of_feature(project: &str, slug: &str) -> SystemPartAnswers {
    let inside = if armada(project) { "bridge" } else { "web" };
    let path = map(project, Some(inside), Some(slug));
    let code: Vec<String> = path.parts.iter().flat_map(|part| part.paths.clone()).collect();
    let features: Vec<SystemFeature> = levels(project).into_iter().flat_map(|level| level.features).filter(|one| one.slug == slug).take(1).collect();
    let base = blank(project, "", armada(project), true);
    if armada(project) {
        return SystemPartAnswers {
            features,
            code,
            working: vec![job("job-metrics", "Live metrics blocks"), job("job-librarian", "Librarian on the dashboard")],
            owners: owner(100),
            docs: vec!["docs/concepts/project.md".into(), "docs/practices/protocol/projects.md".into()],
            monitor: vec![watch("armada check crates/bridge-native:test project_board", "check")],
            issues: armada_issues().into_iter().take(1).collect(),
            pulls: armada_pulls().into_iter().take(1).collect(),
            ..base
        };
    }
    SystemPartAnswers { features, code, owners: owner(100), docs: vec!["SPEC.md".into()], monitor: vec![watch("GET /api/session", "health")], ..base }
}

fn of_part(project: &str, part: &str) -> SystemPartAnswers {
    if armada(project) {
        let (issues, pulls) = (armada_issues(), armada_pulls());
        let base = blank(project, part, true, false);
        return match part {
            "bridge" => SystemPartAnswers {
                code: vec!["crates/bridge-native/".into()],
                working: vec![job("job-metrics", "Live metrics blocks"), job("job-librarian", "Librarian on the dashboard"), job("job-reviews", "Redesign Review Sheet")],
                owners: owner(100),
                docs: vec!["docs/concepts/native-bridge.md".into(), "docs/practices/native-bridge.md".into()],
                monitor: vec![watch("~/Library/Logs/Armada/bridge-native", "log"), watch("native job in checks", "check")],
                issues,
                pulls,
                ..base
            },
            "fleet" => SystemPartAnswers {
                code: vec!["crates/fleet/".into()],
                working: vec![job("job-judge", "Judge feature update"), job("job-metrics", "Live metrics blocks")],
                owners: owner(100),
                docs: vec!["docs/concepts/fleet.md".into(), "ARCHITECTURE.md".into()],
                monitor: vec![watch("GET :40000/health", "health"), watch("~/Library/Logs/Armada/", "log"), watch("scripts/restart", "run")],
                issues: issues.iter().take(1).cloned().collect(),
                pulls,
                ..base
            },
            "api" => SystemPartAnswers {
                code: vec!["crates/ipc/operations/".into(), "crates/fleet/src/session_host/".into()],
                docs: vec!["docs/practices/protocol/".into()],
                monitor: vec![watch("GET :40000/health", "health")],
                pulls: pulls.iter().take(1).cloned().collect(),
                owners: owner(100),
                ..base
            },
            "b_board" => SystemPartAnswers {
                code: vec!["crates/bridge-native/src/project_board/".into(), "crates/fleet/src/project_board.rs".into(), "crates/fleet/src/feature_library/ranking.rs".into()],
                working: vec![job("job-metrics", "Live metrics blocks"), job("job-librarian", "Librarian on the dashboard")],
                owners: owner(100),
                docs: vec!["docs/concepts/project.md".into(), "docs/practices/protocol/projects.md".into()],
                monitor: vec![watch("armada check crates/bridge-native:test project_board", "check")],
                ..base
            },
            "you" | "agents" | "gh" | "cc" | "cf" => base,
            other => SystemPartAnswers { code: paths_of(project, other), owners: owner(100), ..base },
        };
    }
    let base = blank(project, part, false, false);
    match part {
        "web" => SystemPartAnswers {
            code: vec!["frontend/src/screens/".into(), "frontend/src/api.js".into()],
            owners: owner(100),
            docs: vec!["SPEC.md".into()],
            monitor: vec![watch("frontend, in auto_ynab", "run"), watch("npx vite", "server")],
            ..base
        },
        "api" => SystemPartAnswers {
            code: vec!["backend/app/".into()],
            owners: owner(100),
            docs: vec!["SPEC.md".into()],
            monitor: vec![watch("GET /api/session", "health"), watch("uvicorn, --log-level info", "log")],
            ..base
        },
        "you" | "ynab" | "shops" | "bank" => base,
        other => SystemPartAnswers { code: paths_of(project, other), owners: owner(100), ..base },
    }
}

/// One folder of a part's code.
pub fn folder(project: &str, path: &str) -> SystemFolder {
    let names: &[&str] = match path.trim_end_matches('/') {
        "crates/bridge-native" => &["src/", "tests/", "fixtures/", "Cargo.toml", "build.rs"],
        "crates/bridge-native/src/project_board" => &["system/", "board.rs", "layout.rs", "mock.rs", "mod.rs", "presets.rs", "view.rs", "wire.rs"],
        "crates/fleet" => &["src/", "etc/", "Cargo.toml", "build.rs"],
        "frontend" => &["src/", "public/", "package.json", "vite.config.js"],
        "backend/app" => &["main.py", "parsing.py", "persistence.py", "receipts.py", "review.py", "splits.py", "ynab_client.py"],
        _ => &["src/", "Cargo.toml"],
    };
    SystemFolder { project: project.into(), path: path.into(), entries: names.iter().map(|name| name.to_string()).collect() }
}

/// What the librarian adds when he is asked on the mock: one part, kept apart until he keeps it.
pub fn librarian_adds(project: &str) -> SystemMap {
    let mut map = map(project, None, None);
    if !armada(project) {
        return map;
    }
    let mut added = part("docs-site", "Docs site", Entry, "The pages under docs/ read in a browser.", &["docs/"], false);
    added.origin = PartOrigin::Librarian { addition: "docs-site".into(), why: "docs/ holds pages with a build step that nothing else names.".into() };
    map.parts.push(added);
    map.links.push(link("you", "docs-site", ""));
    map
}
