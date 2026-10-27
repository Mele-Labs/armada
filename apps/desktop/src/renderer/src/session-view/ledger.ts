// What a Job has produced, as one list in the order it was produced, for the Completed zone of the story (`StoryColumn.tsx`). Read from what the Job
// already holds: the approved plan, the documents and sketches, the files and pages, the commits in the thread, the walks it ended, the issues
// and pull requests attached, the mods it made and the follow-ups he kept. Nothing is fetched and nothing is guessed from what an agent said.
//
// **The order is the Job's own sequence.** A thread row carries a time and an attachment does not, so the list runs plan, documents, sketches,
// pages, files, commits, walks, mods, issues, pull requests, follow-ups kept; within a kind, oldest first, as the ledger holds them.

import { BadgeCheck, CalendarClock, Footprints, type LucideIcon } from "lucide-react";
import type { LedgerEntry } from "@armada/components";
import { attachmentsOf, type Session } from "@armada/screens/src/draft/sessions";

import { artifactsOf, type ArtifactRow } from "./artifacts";
import { documentOf } from "./document-state";
import { issueOfGroup, type Issue, type PullRequest } from "./IssuesBlock";
import { showLayer } from "./layers";

/** The four lists "By kind" makes. */
export type Bucket = "document" | "pull_request" | "issue" | "artifact";

export const BUCKETS: readonly { id: Bucket; label: string }[] = [
  { id: "document", label: "Documents" },
  { id: "pull_request", label: "Pull requests" },
  { id: "issue", label: "Issues" },
  { id: "artifact", label: "Artifacts" },
];

/** One thing the Job produced. A pull request and an issue are drawn as the ledger draws them; the rest are a row with a glyph. */
export type Made = { key: string; bucket: Bucket; /** The plan group whose issue or pull request it is. */ group?: string } & (
  | { shape: "pr"; pr: PullRequest }
  | { shape: "issue"; issue: Issue }
  | { shape: "row"; row: ArtifactRow; /** The Job's report, drawn with its conclusion. */ report?: true }
  | { shape: "mods" }
);

const row = (key: string, Glyph: LucideIcon, said: string, title: string, open?: () => void): ArtifactRow => ({ key, Glyph, said, title, ...(open === undefined ? {} : { open }) });

/** The report a Job wrote: the first doc or file it made that has a path and somewhere to go. */
export const reportOf = (session: Session, entries: readonly LedgerEntry[]): ArtifactRow | undefined => {
  const groups = artifactsOf(session, entries);
  return [...(groups.find((group) => group.id === "doc")?.rows ?? []), ...(groups.find((group) => group.id === "file")?.rows ?? [])].find((one) => one.sub !== undefined && one.open !== undefined);
};

export function madeOf(session: Session, entries: readonly LedgerEntry[], opts: { landing: string; mods: number; onOpenSketch: (sketch: string) => void }): Made[] {
  const groups = artifactsOf(session, entries);
  // The ledger lists newest first; the story runs oldest first.
  const oldest = (id: string): ArtifactRow[] => [...(groups.find((group) => group.id === id)?.rows ?? [])].reverse();
  const plan = session.work?.plan;
  const out: Made[] = [];
  const made = (bucket: Bucket, one: ArtifactRow, extra: { report?: true } = {}) => out.push({ shape: "row", key: one.key, bucket, row: one, ...extra });

  if (plan?.approved === true) {
    const sketch = plan.sketch;
    made("document", row("plan", BadgeCheck, "Plan", "Plan", sketch === undefined ? undefined : () => opts.onOpenSketch(sketch.id)));
  }
  const report = opts.landing === "report" ? reportOf(session, entries) : undefined;
  for (const one of oldest("doc")) {
    // A page the Job wrote as its document opens in the reading layer over the Job.
    const page = one.sub === undefined ? undefined : documentOf(session, one.sub);
    made("document", page === undefined ? one : { ...one, open: () => showLayer(session.id, "document", page.id) }, one.key === report?.key ? { report: true } : {});
  }
  if (report !== undefined && !out.some((one) => one.key === report.key)) made("document", report, { report: true });
  const planSketch = plan?.sketch?.id;
  for (const one of oldest("sketch")) if (one.key !== `sketch:${planSketch}`) made("document", one);
  for (const one of oldest("page")) made("artifact", one);
  // A file the thread only touched is a code edit, which the commits and pull requests hold; only what the Job wrote as a thing to open is made.
  for (const one of oldest("file")) if (one.key !== report?.key && !one.key.startsWith("touched:")) made("artifact", one);
  for (const one of oldest("commit")) made("artifact", one);
  // A walk ends in his Approve.
  for (const one of session.rows) if (one.kind === "window" && one.ended !== undefined) made("artifact", row(`walk:${one.id}`, Footprints, "Walk approved", one.title));
  if (opts.mods > 0) out.push({ shape: "mods", key: "mods", bucket: "artifact" });

  const groupOfIssue = (number: string) => plan?.groups.find((one) => one.issue === number)?.id;
  for (const issue of attachmentsOf(session, "issue")) {
    const group = groupOfIssue(issue.number);
    out.push({ shape: "issue", key: `issue:${issue.number}`, bucket: "issue", issue, ...(group === undefined ? {} : { group }) });
  }
  for (const pr of attachmentsOf(session, "pull_request")) {
    const group = plan?.groups.find((one) => issueOfGroup(session, one.issue)?.prs.some((other) => other.number === pr.number) === true)?.id;
    out.push({ shape: "pr", key: `pr:${pr.number}`, bucket: "pull_request", pr, ...(group === undefined ? {} : { group }) });
  }
  // A follow-up he marked Deferred is settled; the rest are still to come (`StoryColumn.tsx`, Next).
  for (const one of session.later ?? []) if (one.confirmed === true) made("issue", row(`later:${one.id}`, CalendarClock, "Deferred", one.text));
  return out;
}

/** What each kind of output is made by, as the name of a workflow step reads. Until Fleet records the step that made an output, the step is the one named for it. */
/** The workflow step that shows a walk. */
export const WALK_STEP = /present|walk|show/i;

const MADE_BY: readonly { is: (one: Made) => boolean; step: RegExp; /** Where it stands when no step is named for it. */ first?: true }[] = [
  { is: (one) => one.shape === "row" && one.key.startsWith("walk:"), step: WALK_STEP },
  { is: (one) => one.shape === "row" && one.bucket === "document", step: /plan|draft|design|scope/i, first: true },
  { is: (one) => one.shape === "pr", step: /pull|deliver|land/i },
];

/** The finished workflow step an output stands under in By step, or none where no finished step made it. */
export const stepOf = (one: Made, done: readonly string[]): string | undefined => {
  const by = MADE_BY.find((kind) => kind.is(one));
  return by === undefined ? undefined : (done.find((name) => by.step.test(name)) ?? (by.first === true ? done[0] : undefined));
};

/** Each kind's list, newest first. A kind with nothing made has no list. */
export const byKind = (list: readonly Made[]): { id: Bucket; label: string; made: Made[] }[] =>
  BUCKETS.flatMap(({ id, label }) => {
    const mine = list.filter((one) => one.bucket === id);
    return mine.length === 0 ? [] : [{ id, label, made: [...mine].reverse() }];
  });
