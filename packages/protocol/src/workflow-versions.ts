// The versions a workflow has had in one place, and what a copy of one of Armada's began from: field for field with
// `crates/ipc/src/workflow_versions.rs`. Protocol 23.96,
// `docs/practices/protocol/protocol-23-96-a-workflow-with-versions-and-a-base.md`.
//
// A version is a whole text, written by a publish. Going back is `POST /workflows/revert`, which puts that text in the draft and publishes nothing.

/** How a version came to be kept: a publish wrote it, or it was the text in the place when the first publish came (version 1). */
export type VersionOrigin = "published" | "found";

/** One version of a workflow in one place. */
export type WorkflowVersion = {
  /** From 1, in the order they were written. */
  version: number;
  /** The definition, whole. */
  text: string;
  /** The sha256 of `text`, in hex. */
  hash: string;
  at: string;
  origin: VersionOrigin;
};

/**
 * `GET /workflows/versions?workflow_id=&source=&project=&manifest_id=`: newest first, and empty where nothing was published in that place. `source` is
 * `kit`, `repository` or `project`; 422 `fleet.workflow_project_unnamed`, 404 `fleet.workflow_project_unknown` and 422 `fleet.workflow_project_not_member` for a project's.
 */
export type WorkflowVersions = {
  versions: WorkflowVersion[];
};

/**
 * `POST /workflows/revert?manifest_id=`'s body: puts that version's text in the draft as a change by `you` with the summary `Back to version N`, beginning
 * the draft where there is none. Answers the `WorkflowDraft`. Nothing is published. 404 `fleet.workflow_version_none` where the place has no such version.
 */
export type RevertWorkflow = {
  workflow_id: string;
  source: string;
  /** Required where `source` is `project`. */
  project?: string;
  version: number;
};

/** What a copy of one of Armada's workflows began from: the `version` Armada's definition declared then, and the sha256 of its text. Absent on a copy made before Fleet kept it. */
export type BasedOn = {
  version: number;
  /** The sha256 of the carried text, in hex. */
  hash: string;
};
