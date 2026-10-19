//! A pull request's patches, every file's or one file's.
//!
//! `gh api .../pulls/N/files` is the only forge read that carries a patch per file, and the
//! forge leaves `patch` out of a file it will not show (binary, or too large). `gh pr view`
//! has no patch and `gh pr diff` is one document cut at 20,000 lines, so neither says which
//! file lacks one. Read as `crate::pull_request_detail` reads its own: jq prints a record per
//! line, and no document is parsed here.

use adapter_traits::{FilePatch, PatchedFile};

use crate::delivery::run_in;
use crate::under_review::as_written;

/// `+` where the forge sent a patch and `-` where it left it out, so an empty patch and an
/// absent one are not the same record.
const REDUCTION: &str =
    r#".[] | [(.filename // ""), (if .patch == null then "-" else "+" end), (.patch // "")] | @tsv"#;

/// `None` is the forge's silence: no tool, nobody signed in, no such pull request.
pub(crate) fn read_every(in_repo: &str, pull_request: &str) -> Option<Vec<PatchedFile>> {
    // The number goes into the endpoint, so it is a number or nothing.
    if pull_request.is_empty() || !pull_request.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let endpoint = format!("repos/{{owner}}/{{repo}}/pulls/{pull_request}/files?per_page=100");
    let run = run_in(in_repo, "gh", &["api", "--paginate", &endpoint, "--jq", REDUCTION]).ok()?;
    if !run.status.success() {
        return None;
    }
    let lines: Vec<String> = String::from_utf8_lossy(&run.stdout)
        .lines()
        .map(str::to_string)
        .collect();
    Some(listed(&lines))
}

/// One file's patch, out of [`read_every`]'s one read.
pub(crate) fn read(in_repo: &str, pull_request: &str, path: &str) -> Option<FilePatch> {
    read_every(in_repo, pull_request).map(|files| folded(files, path))
}

/// The records jq printed, in its order. Split from [`read_every`] so a test hands it what jq
/// prints and needs no forge.
pub(crate) fn listed(lines: &[String]) -> Vec<PatchedFile> {
    lines
        .iter()
        .filter_map(|line| {
            let mut field = line.splitn(3, '\t').map(as_written);
            let (Some(path), Some(sent), Some(patch)) = (field.next(), field.next(), field.next())
            else {
                return None;
            };
            Some(PatchedFile {
                path,
                patch: (sent == "+").then_some(patch),
            })
        })
        .collect()
}

/// The files listed, reduced to the one asked for.
pub(crate) fn folded(files: Vec<PatchedFile>, path: &str) -> FilePatch {
    match files.into_iter().find(|file| file.path == path) {
        Some(PatchedFile { patch: Some(patch), .. }) => FilePatch::Patch(patch),
        Some(PatchedFile { patch: None, .. }) => FilePatch::NoPatch,
        None => FilePatch::NoSuchFile,
    }
}
