// What the side panel draws from: one read for each kind of thing it opens, served by the window. **Filled from the
// forge and from Fleet, not from the surface that opened it**, so a pull request opens the same from the merge queue
// as from a Job's ledger. The mock's reads are fixtures (`mock/shared-panel-reads.ts`); no Fleet serves them yet.

import { createContext, useContext, useEffect, useState } from "react";
import type { ReactNode } from "react";
import type { MergeLines, PullRequestFilePatches, PullRequestPage, WorktreeSlot } from "@armada/protocol";
import type { PullRequestAct, SessionAttachment } from "@armada/screens/src/draft/sessions";

import { browsePullRequestsWith, forgetPanel, hostsPanel } from "./store";
import type { PanelItem } from "./store";

type Of<K extends PanelItem["kind"]> = Extract<PanelItem, { kind: K }>;

/** A pull request as the forge says it, whoever owns it. */
export type PullRequestRead = {
  repository: string;
  root?: string;
  /** The state the panel draws, as a Session's attachment carries it. */
  facts: Extract<SessionAttachment, { kind: "pull_request" }>;
  page: PullRequestPage;
};

export type IssueRead = {
  number: number;
  title: string;
  repository: string;
  address: string;
  state: "open" | "closed";
  author: string;
  openedAt: string;
  labels: readonly string[];
  assignees: readonly string[];
  body: string;
  comments: readonly { author: string; at: string; body: string }[];
  /** Pull requests that name it, each opening in this same panel. */
  linked: readonly { number: number; title: string; address: string; state: "open" | "merged" | "closed" }[];
};

export type BayRead = {
  name: string;
  repository: string;
  state: "leased" | "free" | "stranded" | "closed";
  word: string;
  holder?: { id: string; title: string };
  branch?: string;
  heldFor?: string;
  base: string;
  behind: number;
  warm: boolean;
  path: string;
  uncommitted: readonly string[];
  unpushed: readonly string[];
  history: readonly { who: string; span: string; branch?: string }[];
  /** The pool's own slot, where Fleet served it: the panel manages the bay through it. */
  slot?: WorktreeSlot;
};

export type CheckRead = {
  name: string;
  repository: string;
  state: "passed" | "failed" | "running";
  runs: readonly { on: string; state: "passed" | "failed" | "running"; seconds?: number; ago: string; pull?: { number: number; address: string } }[];
  log: { run: string; lines: readonly string[] };
};

/** Every changed file's patch in the forge's order, or Fleet's words for why they were not read. */
export type PatchesRead = { ok: true; files: PullRequestFilePatches["files"] } | { ok: false; said: string };

export type PanelReads = {
  pullRequest: (item: Of<"pr">) => Promise<PullRequestRead | undefined>;
  /** Every file's patch, read once the first time the Files tab is shown. Absent leaves the Files tab a list. */
  pullRequestPatches?: (item: Of<"pr">) => Promise<PatchesRead>;
  /** A comment or a review written on the forge. Answers the page as it stands after. */
  writePullRequest?: (item: Of<"pr">, write: { kind: "comment" | "approve" | "request_changes"; body?: string }) => Promise<{ ok: true; page: PullRequestPage } | { ok: false; said: string }>;
  /** A forge act on a pull request nobody here owns. A pull request a Job holds goes through that Job. */
  actOnPullRequest?: (item: Of<"pr">, act: PullRequestAct) => Promise<boolean>;
  issue: (item: Of<"issue">) => Promise<IssueRead | undefined>;
  bay: (item: Of<"bay">) => Promise<BayRead | undefined>;
  check: (item: Of<"check">) => Promise<CheckRead | undefined>;
  /** The Check panel closed: stop following the log it was reading. */
  leaveCheck?: () => void;
  /** Moves each time the log being followed does, so a Check panel reads its lines again. */
  followed?: number;
  /** Takes a pull request's address where `links.openPullRequestsIn` sends them to the browser, and answers whether it did. */
  browse?: (address: string) => boolean;
  /** What the merge lines say of the place a pull request holds. */
  lines: MergeLines | null;
  /** The issues and pull requests a `#number` in the panel's words can name. */
  numbered: readonly { kind: "issue" | "pull_request"; number: string; title: string; url?: string }[];
};

const Held = createContext<PanelReads | undefined>(undefined);

/** Hands the window its reads, and says a panel is hosted for as long as it is mounted. */
export function SharedPanelFrom({ reads, children }: { reads: PanelReads | undefined; children: ReactNode }) {
  const hosted = reads !== undefined;
  useEffect(() => {
    if (!hosted) return;
    forgetPanel();
    return hostsPanel();
  }, [hosted]);
  const browse = reads?.browse;
  useEffect(() => browsePullRequestsWith(browse), [browse]);
  return <Held.Provider value={reads}>{children}</Held.Provider>;
}

export const usePanelReads = (): PanelReads | undefined => useContext(Held);

/** A read as it stands: out, or answered, where an answer of nothing is `value` absent. */
export type Reading<T> = { out: true } | { out: false; value: T | undefined };

/** A read, started when `key` changes. A change of `again` reads once more and keeps what was read meanwhile. A read that throws answered nothing. */
export function useReading<T>(start: () => Promise<T | undefined>, key: string, again = 0): Reading<T> {
  const [value, setValue] = useState<{ key: string; value: T | undefined } | undefined>(undefined);
  useEffect(() => {
    let current = true;
    void start()
      .catch(() => undefined)
      .then((answer) => {
        if (current) setValue({ key, value: answer });
      });
    return () => {
      current = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, again]);
  return value?.key === key ? { out: false, value: value.value } : { out: true };
}

/** `useReading`, absent while it is in flight and where nothing was found. */
export function useRead<T>(start: () => Promise<T | undefined>, key: string, again = 0): T | undefined {
  const reading = useReading(start, key, again);
  return reading.out ? undefined : reading.value;
}
