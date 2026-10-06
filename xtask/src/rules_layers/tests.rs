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
fn layers_run_screens_surfaces_desktop() {
    let s = surfaces();
    assert_eq!(layer_of("@armada/screens", &s), Some(3));
    assert_eq!(layer_of("@armada/jobs", &s), Some(4));
    assert_eq!(layer_of("@armada/desktop", &s), Some(5));
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

#[test]
fn surfaces_on_disk_is_empty_without_the_directory() {
    assert!(surfaces_on_disk(Path::new("/nonexistent-armada-root")).is_empty());
}
