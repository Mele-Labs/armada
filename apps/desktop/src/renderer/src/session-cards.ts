// What a Session ledger row's card says: built from the attachment and what the window already holds,
// so `SessionLedger` stays a drawing and nothing is read for a hover.

import { FILE_SHOWN, type SessionLedgerCard } from "@armada/components";
import { GitMerge, GitPullRequest } from "lucide-react";
import type { WorktreeSlot } from "@armada/protocol";
import { attachmentsOf, issueLabel } from "@armada/screens/src/draft/sessions";
import type { Session, SessionAttachment } from "@armada/screens/src/draft/sessions";

import { backgroundCardOf } from "./session-background";

/** What is true of a pull request beyond its Checks, as bare facts. */
export const factsOf = (one: Extract<SessionAttachment, { kind: "pull_request" }>): string[] =>
  [one.state === "draft" ? "Draft" : undefined, one.queued === true ? "In merge queue" : one.auto && one.state !== "merged" ? "Auto-merge on" : undefined].filter(
    (fact): fact is string => fact !== undefined,
  );

/** A pull request's Checks in words, the tooltip on its Checks mark: the Linked rail's and Progress's. */
export const checksSaid = (checks: Extract<SessionAttachment, { kind: "pull_request" }>["checks"]): string => {
  if (checks.state === "failed") return `Checks failed: ${checks.failing}`;
  const red = checks.red === undefined ? "" : checks.state === "pending" ? `; failing: ${checks.red}` : `; not required, failing: ${checks.red}`;
  return `${checks.state === "pending" ? "Checks running" : "Checks passed"}${red}`;
};

/** A file the agent showed and did not write. */
export const shownOnly = (one: { form: string; by?: "agent"; written?: true }): boolean => one.form === "file" && one.by === "agent" && one.written !== true;

export const ARTIFACT_SAID = { page: "Published page", file: "File written", image: "Looked at", doc: "Doc", document: "Document", window: "Shown in a window" } as const;

/** What else the window already holds that a row's card can say: the slots Cleanup read. Nothing is read for a card. */
export type Known = { slots: readonly WorktreeSlot[] };

const PR_STATE = { open: "Open", draft: "Draft", merged: "Merged" } as const;

/** A pull request's card: the ledger row's and the Sessions list chip's, so the two say the same. */
export function pullRequestCardOf(one: Extract<SessionAttachment, { kind: "pull_request" }>): SessionLedgerCard {
  return {
    title: one.title,
    where: `#${one.number}`,
    state: { said: [PR_STATE[one.state], ...factsOf(one).filter((fact) => fact !== "Draft")].join(", "), Glyph: one.state === "merged" ? GitMerge : GitPullRequest, ...(one.state === "merged" ? { tone: "passed" as const } : {}) },
    facts: [
      one.checks.state === "failed"
        ? { label: "Checks", value: `Failed: ${one.checks.failing}`, tone: "failed" }
        : one.checks.state === "pending"
          ? { label: "Checks", value: "Running", tone: "pending" }
          : { label: "Checks", value: "Passed", tone: "passed" },
      { label: "Branch", value: one.branch, mono: true },
    ],
  };
}

/** What hovering a row shows beside it, from what the attachment and the window already hold. */
export function cardOf(one: SessionAttachment, session: Session, sessions: readonly Session[], known: Known): SessionLedgerCard {
  switch (one.kind) {
    case "pull_request":
      return pullRequestCardOf(one);
    case "slot": {
      const branch = attachmentsOf(session, "branch").find((b) => b.slot === one.slot)?.name ?? known.slots.find((s) => s.slot === one.slot)?.branch;
      return {
        title: `Worktree slot ${one.slot}`,
        facts: [
          ...(branch === undefined ? [] : [{ label: "Branch", value: branch, mono: true }]),
          { label: "Held by", value: "This Job" },
        ],
      };
    }
    case "branch": {
      const behind = known.slots.find((s) => s.branch === one.name && s.behind !== undefined);
      return {
        title: one.name,
        facts: [
          { label: "Slot", value: String(one.slot) },
          ...(behind === undefined ? [] : [{ label: `Behind ${behind.base}`, value: String(behind.behind) }]),
        ],
      };
    }
    case "background":
      return backgroundCardOf(one);
    case "subagent":
      return { title: one.task, facts: [{ label: "Turn", value: one.state === "running" ? "Running" : "Done", tone: one.state }] };
    case "artifact":
      return { title: one.title, facts: [{ label: "Form", value: shownOnly(one) ? FILE_SHOWN : ARTIFACT_SAID[one.form] }, { label: one.form === "file" || one.form === "image" || one.form === "document" ? "Path" : "Address", value: one.id, mono: true }] };
    case "issue":
      return { title: one.title, where: issueLabel(one.number), facts: [{ label: "State", value: one.state === "open" ? "Open" : "Closed" }] };
    case "studio":
      return { title: one.title, facts: [{ label: "Kind", value: "Studio" }] };
    case "link":
      return { title: one.url, facts: [{ label: "Written by", value: one.by === "agent" ? "The agent" : "You" }] };
    case "sketch":
      return { title: one.title, facts: [{ label: "Kind", value: "Sketch" }, { label: "Drawn by", value: one.by === "agent" ? "The agent" : "You" }] };
    case "started_to":
    case "started_by": {
      const other = sessions.find((s) => s.id === one.id);
      return {
        title: other?.title ?? one.id,
        facts: [
          { label: "Kind", value: one.kind === "started_to" ? "Job this one's agent started" : "Job whose agent started this one" },
          ...(other?.address === undefined ? [] : [{ label: "Address", value: other.address, mono: true }]),
        ],
      };
    }
    case "forked_to":
    case "forked_from": {
      const other = sessions.find((s) => s.id === one.id);
      return {
        title: other?.title ?? one.id,
        facts: [
          { label: "Kind", value: one.kind === "forked_to" ? "Job forked from this one" : "Job this one was forked from" },
          ...(other?.address === undefined ? [] : [{ label: "Address", value: other.address, mono: true }]),
        ],
      };
    }
  }
}
