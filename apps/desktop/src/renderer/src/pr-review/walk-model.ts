// What the Walk tool holds. **A walk is Armada's own source**: it writes the steps from the pull request and plays them
// against the change running somewhere. Where it runs is found in his order, one source at a time: the pull request
// itself (a preview link a deploy bot posted, or a deploy check's address), then what Armada remembers of this
// repository and the area of code the pull request touches, and only then him. Mock only: nothing here is served.

import type { PullRequestEntry } from "@armada/protocol";

import type { ReviewPr } from "./model";

/** Where the change runs: already deployed, or pulled down into a slot and run. */
export type Place = {
  how: "deployed" | "pull";
  /** The web address, and for a pulled-down change the one Armada waits on before it walks. */
  address: string;
  /** What starts the change, where it is pulled down. */
  command: string;
  /** The API's base address, where there is an API. */
  api: string;
};

/** A place kept for one area of one repository, so the next pull request there does not ask. */
export type Remembered = { id: string; repository: string; area: string; place: Place };

/** The order he chose. Each is tried only when the one before found nothing. */
export type Source = "pr" | "memory" | "ask";

export type SourceState = "looking" | "found" | "nothing" | "asking";

/** One line of how the place was found: the source tried, what it came to, and the words. */
export type Tried = { source: Source; state: SourceState; said: string; by?: string };

/** One step of a web walk, in the shape `show_window` steps have: a verb on a target found by role and name. */
export type WebStep = { id: string; say: string; verb: "press" | "look" | "hover" | "type" | "key"; role: string; name: string; with?: string };

/** One request of an API walk, with the response the mock gets back and what to look for in it. */
export type ApiStep = {
  id: string;
  say: string;
  method: "GET" | "POST" | "PUT" | "DELETE";
  path: string;
  body?: string;
  response: { status: number; ms: number; body: string };
  look: string;
};

/** What the agent wrote from the pull request: its description, How to Test and its themes. */
export type WalkPlan =
  | { kind: "web"; steps: readonly WebStep[]; slot: number }
  | { kind: "api"; steps: readonly ApiStep[]; slot: number };

export type WalkSeed = { plans: Readonly<Record<string, WalkPlan>>; memory: readonly Remembered[] };

/** A state a setup line or a request can be in. */
export type Phase = "pending" | "running" | "done" | "failed";

/** What is kept of a walk for one pull request, for the commit it was written on. */
export type WalkState = {
  head: string;
  tried: readonly Tried[];
  place: Place | undefined;
  setup: readonly { id: string; said: string; code?: string; phase: Phase }[];
  /** The requests that have been sent, by id. */
  sent: Readonly<Record<string, Phase>>;
  /** The walk window is up, and the step it is on. */
  window: number | undefined;
};

/** The words of a place, for a line and a row. */
export function placeSaid(place: Place): string {
  const api = place.api === "" ? "" : `API at ${place.api}`;
  if (place.how === "deployed") return [`Deployed at ${place.address}`, api].filter((one) => one !== "").join(", ");
  return [`Pull it down, run ${place.command}`, place.address === "" ? "" : `at ${place.address}`, api].filter((one) => one !== "").join(", ");
}

/** What a pulled-down place waits on before it walks. */
export const waitsOn = (place: Place): string => (place.address === "" ? place.api : place.address);

/**
 * The area of code a pull request touches: **the directory, two deep, that most of its files are in**, and where files
 * tie the one with more changed lines. `crates/fleet` for a change to Fleet, `apps/desktop` for one to Bridge.
 */
export function areaOf(pr: Pick<ReviewPr, "page">): string {
  const by = new Map<string, { files: number; lines: number }>();
  for (const file of pr.page.files) {
    const parts = file.path.split("/");
    const area = parts.length > 2 ? parts.slice(0, 2).join("/") : (parts[0] ?? "");
    const one = by.get(area) ?? { files: 0, lines: 0 };
    by.set(area, { files: one.files + 1, lines: one.lines + file.additions + file.deletions });
  }
  return [...by.entries()].sort((a, b) => b[1].files - a[1].files || b[1].lines - a[1].lines)[0]?.[0] ?? "";
}

/** An area as it is shown: a directory, with its slash. */
export const areaShown = (area: string): string => (area === "" || area.endsWith("/") ? area : `${area}/`);

/** What Armada remembers of where changes to this area of this repository run. */
export const rememberedFor = (memory: readonly Remembered[], pr: ReviewPr): Remembered | undefined => {
  const area = areaOf(pr);
  return memory.find((one) => one.repository === pr.repository && areaShown(area).startsWith(areaShown(one.area)));
};

const ADDRESS = /https?:\/\/[^\s)>\]"']+/;

const botOf = (entry: PullRequestEntry): boolean => entry.kind === "comment" && /bot$|\[bot\]$/i.test(entry.author);

/**
 * What the pull request itself says about where it runs: a preview a deploy bot posted in the conversation, or the
 * address of a check that deploys. Nothing else on the forge is read for it.
 */
export function fromPullRequest(pr: ReviewPr): { address: string; said: string; by: string } | undefined {
  for (const entry of pr.page.conversation) {
    if (!botOf(entry)) continue;
    const found = /(preview|deployed)[^\n]*?(https?:\/\/[^\s)>\]"']+)/i.exec(entry.body);
    if (found?.[2] !== undefined) return { address: found[2], said: entry.body.split("\n")[0] ?? found[2], by: entry.author };
  }
  for (const check of pr.page.checks) {
    if (check.state === "passed" && /preview|deploy/i.test(check.name) && check.address !== undefined && ADDRESS.test(check.address)) {
      return { address: check.address, said: `${check.name}: ${check.address}`, by: check.name };
    }
  }
  return undefined;
}
