// The builds of Bridge the Usage page reads its figures against, as the Builds panel and both views' narrowing draw them. These
// types are the page's own: the window turns Fleet's builds into them (`usage-time-wire.ts`).

import type { UsageBreakdown } from "@armada/protocol";

import { dayName } from "./usage-time";
import type { TimeSplit } from "./usage-time";
import { percentDelta } from "./usage-view";

/** One build, with what the page narrows to when it is picked. */
export type Build = {
  /**
   * Tells the build from every other: its own short commit, then where it began, since a commit run again later is a second
   * build. Read the commit off it with `commitOf`.
   */
  id: string;
  /** The main commit the build stands on. */
  commit: string;
  /** When the build began running, as an instant. */
  started: string;
  /** The branches a preview build holds. Empty for a build on main. */
  branches: readonly string[];
  /** Turns taken while the build ran. */
  turns: number;
  /** Cost units per turn. Absent where the build took no turn. */
  burn?: number;
  /** Seconds: the median wait on the model per turn, a tool call in the main thread, a Check run, and a pull request's wait. Each absent where the build produced none. */
  model?: number;
  tools?: number;
  checks?: number;
  pull_request?: number;
  /** The Jobs that ran on it, by id. */
  jobs: readonly string[];
  /** What the Tokens view shows when the build is picked. Absent until it has been read. */
  usage?: UsageBreakdown;
  /** What the Time view shows: the split of the build's Jobs together. */
  split: TimeSplit;
};

/** Newest first. */
export type UsageBuilds = readonly Build[];

/** A build's commit as the page draws it: the first seven characters of its id. */
export const commitOf = (build: Build): string => build.id.slice(0, 7);

/** A build under this many turns has too few to read against the one before. */
export const TOO_FEW = 30;

/** A move under this share of the figure before reads as no change. */
export const STILL = 5;

export type Figure = "burn" | "model" | "tools" | "checks" | "pull_request";

export const TIME_FIGURES = [
  "model",
  "tools",
  "checks",
  "pull_request",
] as const;

export const FIGURE_NAME: Readonly<Record<Figure, string>> = {
  burn: "Burn per turn",
  model: "Model per turn",
  tools: "Tools per call",
  checks: "Checks per run",
  pull_request: "Pull request wait",
};

/** A column's head: the figure with its unit. */
export const FIGURE_HEAD: Readonly<Record<Figure, string>> = {
  burn: "Burn / turn",
  model: "Model / turn",
  tools: "Tools / call",
  checks: "Checks / run",
  pull_request: "Pull request",
};

export const FIGURE_MEANS: Readonly<Record<Figure, string>> = {
  burn: "Cost units per turn",
  model: "The median wait on the model's answer, per turn",
  tools: "The median time a tool ran in the main thread, per call",
  checks: "The median time a Check queued or ran, per run",
  pull_request: "The median wait in CI and the merge queue with nothing else running, per pull request",
};

/** A figure as a reader takes it: `5.2s`, `41s`, `8.6m`, and whole minutes from a quarter hour. */
export function lapse(seconds: number): string {
  if (seconds < 10) return `${seconds.toFixed(1)}s`;
  if (seconds < 60) return `${Math.round(seconds)}s`;
  if (seconds < 900) return `${(seconds / 60).toFixed(1)}m`;
  const minutes = Math.round(seconds / 60);
  return minutes < 60 ? `${minutes}m` : `${Math.floor(minutes / 60)}h ${String(minutes % 60).padStart(2, "0")}m`;
}

const part = (n: number): string => String(n).padStart(2, "0");

/** When a build began, in the reader's own clock: `Nov 3 14:36`. */
export function startedAt(instant: string): string {
  const at = new Date(instant);
  return `${dayName(`${at.getFullYear()}-${part(at.getMonth() + 1)}-${part(at.getDate())}`)} ${part(at.getHours())}:${part(at.getMinutes())}`;
}

/** What hovering a build says: its commit, and for a preview what it holds. */
export const buildTip = (build: Build): string =>
  build.branches.length > 0
    ? `Preview of main ${build.commit} with ${build.branches.join(", ")}`
    : `Main ${build.commit}`;

/** Burn per turn in thousands of units. */
export const thousands = (units: number): string => `${Math.round(units / 1000)}k`;

/** Absent where the build produced none of the figure. */
export const valueOf = (build: Build, figure: Figure): number | undefined =>
  figure === "burn" ? build.burn : build[figure];

export const shown = (build: Build, figure: Figure): string => {
  const value = valueOf(build, figure);
  if (value === undefined) return "–";
  return figure === "burn" ? thousands(value) : lapse(value);
};

/** The change in a figure against the build before it. Absent where either has too few turns to compare, or lacks the figure. */
export function changeOf(
  build: Build,
  before: Build | undefined,
  figure: Figure,
): { up: boolean; percent: number; still: boolean } | undefined {
  if (before === undefined || build.turns < TOO_FEW || before.turns < TOO_FEW) return undefined;
  const now = valueOf(build, figure);
  const was = valueOf(before, figure);
  if (now === undefined || was === undefined) return undefined;
  const moved = percentDelta(now, was);
  if (moved === undefined) return { up: false, percent: 0, still: true };
  return { ...moved, still: moved.percent < STILL };
}

/** The keys the list takes itself while it holds focus; the page's own listeners leave them. */
export const LIST_KEYS = [
  "ArrowDown",
  "ArrowUp",
  "j",
  "k",
  "Enter",
  "o",
];

export const inBuilds = (target: EventTarget | null): boolean =>
  target instanceof HTMLElement && target.closest("[data-builds]") !== null;
