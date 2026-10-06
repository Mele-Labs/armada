//! The surface layer's negative cases, proved on specifiers written here.

use super::*;

fn surfaces() -> Vec<String> {
    vec!["@armada/jobs".to_string(), "@armada/studios".to_string()]
}

fn fault(path: &str, spec: &str) -> Option<String> {
    import_fault(
        path,
        "@armada/jobs",
        "packages/surfaces/jobs",
        spec,
        &surfaces(),
    )
}

#[test]
fn layers_run_screens_bridge_api_surfaces_desktop() {
    let s = surfaces();
    assert_eq!(layer_of("@armada/screens", &s), Some(3));
    assert_eq!(layer_of("@armada/bridge-api", &s), Some(4));
    assert_eq!(layer_of("@armada/jobs", &s), Some(5));
    assert_eq!(layer_of("@armada/desktop", &s), Some(6));
    assert_eq!(layer_of("@armada/other", &s), None);
}

#[test]
fn a_surface_importing_screens_passes() {
    assert_eq!(
        fault("packages/surfaces/jobs/src/Jobs.tsx", "@armada/screens"),
        None
    );
}

#[test]
fn a_surface_importing_a_sibling_fails() {
    assert!(fault("packages/surfaces/jobs/src/Jobs.tsx", "@armada/studios").is_some());
    assert!(fault(
        "packages/surfaces/jobs/src/Jobs.test.tsx",
        "@armada/studios"
    )
    .is_some());
}

#[test]
fn a_surface_importing_desktop_from_source_fails() {
    assert!(fault(
        "packages/surfaces/jobs/src/Jobs.tsx",
        "@armada/desktop/mock"
    )
    .is_some());
}

#[test]
fn a_surface_test_importing_desktop_passes() {
    assert_eq!(
        fault(
            "packages/surfaces/jobs/src/Jobs.test.tsx",
            "@armada/desktop/mock"
        ),
        None
    );
    assert_eq!(
        fault(
            "packages/surfaces/jobs/src/model.test.ts",
            "@armada/desktop/mock"
        ),
        None
    );
    assert_eq!(
        fault(
            "packages/surfaces/jobs/vitest.config.ts",
            "@armada/desktop/vitest-preset"
        ),
        None
    );
}

#[test]
fn the_exemption_is_for_surfaces_only() {
    let s = surfaces();
    let screens_test = import_fault(
        "packages/screens/src/Board.test.tsx",
        "@armada/screens",
        "packages/screens",
        "@armada/desktop/mock",
        &s,
    );
    assert!(screens_test.is_some());
}

fn from_bridge_api(spec: &str) -> Option<String> {
    import_fault(
        "packages/bridge-api/src/index.ts",
        "@armada/bridge-api",
        "packages/bridge-api",
        spec,
        &surfaces(),
    )
}

#[test]
fn bridge_api_imports_protocol_and_screens() {
    assert_eq!(from_bridge_api("@armada/protocol"), None);
    assert_eq!(
        from_bridge_api("@armada/screens/src/fixtures/fixture"),
        None
    );
}

#[test]
fn bridge_api_does_not_import_up() {
    assert!(from_bridge_api("@armada/jobs").is_some());
    assert!(from_bridge_api("@armada/desktop").is_some());
}

#[test]
fn a_surface_imports_bridge_api() {
    assert_eq!(
        fault("packages/surfaces/jobs/src/Jobs.tsx", "@armada/bridge-api"),
        None
    );
}

#[test]
fn surfaces_on_disk_is_empty_without_the_directory() {
    assert!(surfaces_on_disk(Path::new("/nonexistent-armada-root")).is_empty());
}
