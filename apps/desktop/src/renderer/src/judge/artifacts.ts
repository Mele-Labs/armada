// Which artifact a ledger row is to a judge, and where a reading is opened at its notes.

import { attachmentsOf, type Session } from "@armada/screens/src/draft/sessions";

import { openInPanel } from "../shared-panel/store";
import { documentOf } from "../session-view/document-state";
import { showLayer } from "../session-view/layers";
import type { JudgeArtifact, JudgeKind } from "./store";

/** The artifact kind of a ledger group, for the rows whose key is the address or path. */
const BY_GROUP: Readonly<Record<string, JudgeKind>> = { page: "page", file: "file", doc: "doc" };

/** The artifact a Artifacts-layer row stands for, from its key and group. Absent for a row a judge has nothing to read in (a link, a Job, a closed item). */
export function artifactOfRow(row: { key: string; title: string; sub?: string | undefined }, group: string): JudgeArtifact | undefined {
  const at = row.key.indexOf(":");
  const prefix = row.key.slice(0, at);
  const id = row.key.slice(at + 1);
  const make = (kind: JudgeKind, title = row.title): JudgeArtifact => ({ kind, id, title });
  switch (prefix) {
    case "pr":
      return make("pull_request", `#${id}`);
    case "commit":
      return make("commit");
    case "artifact":
      return BY_GROUP[group] === undefined ? undefined : make(BY_GROUP[group]!);
    case "touched":
      return make("file");
    case "sketch":
      return make("sketch");
    case "ran":
    case "bash":
    case "background":
      return make("command");
    case "subagent":
      return make("subagent");
    case "issue":
      return make("issue");
    case "note":
      return make("note");
    default:
      return undefined;
  }
}

/** Opens the artifact at its notes: a pull request on its Files tab, a document in the reading layer, the plan on the card; any other on the Job's artifacts. */
export function openAtNotes(session: Session, artifact: Pick<JudgeArtifact, "kind" | "id">): void {
  if (artifact.kind === "pull_request") {
    const pr = attachmentsOf(session, "pull_request").find((one) => String(one.number) === artifact.id);
    if (pr !== undefined) return void openInPanel({ kind: "pr", number: pr.number, address: pr.address, tab: "files", ...(session.manifestId === undefined ? {} : { manifestId: session.manifestId }) });
  }
  if (artifact.kind === "doc") {
    const page = documentOf(session, artifact.id);
    if (page !== undefined) return showLayer(session.id, "document", page.id);
  }
  if (artifact.kind === "plan") {
    const note = document.querySelector<HTMLElement>(".armada-session-thread__plan [data-finding]");
    if (note !== null) {
      note.scrollIntoView({ block: "center" });
      return note.focus();
    }
  }
  showLayer(session.id, "artifacts");
}
