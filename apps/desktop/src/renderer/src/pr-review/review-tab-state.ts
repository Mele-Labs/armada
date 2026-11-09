// What the Review tab holds for each open pull request, kept outside the tab so a key, a peek's Jump and Go to file can move it from
// anywhere: the file or tour stop it reads, the finding in focus, the lines picked for a comment and how each finding was marked. Kept for the
// window's life, not on disk: a mark is his reading of one run, and nothing on the forge sees it.

import { useSyncExternalStore } from "react";
import type { DiffSide, ReviewPlace } from "@armada/protocol";

import { CONTEXT, counted } from "./patch";
import type { DiffLine } from "@armada/components";
import type { Finding, PrWork, ReviewPr } from "./model";
import { TOUR_RUN, tourOf } from "./runs";

/** How he marked a finding. Any mark puts it away. */
export type FindingMark = "expected" | "not-valid" | "commented";

/** What the Review tab is asked to bring into view next. A new `n` each ask, so asking for the same place twice moves there twice. */
export type Wanted = {
  n: number;
  finding?: string;
  line?: {
    line: number;
    side: DiffSide;
  };
};

export type ReviewTabState = {
  /** The file the Review tab reads. Absent: the first in its order, or on a tour the stop it is at. */
  path: string | undefined;
  /** The stop of the tour it is at, where no file was asked for. Absent: the first. */
  stop: number | undefined;
  /** The stop's whole file is drawn, not only its lines. */
  whole: boolean;
  finding: string | undefined;
  /** Lines of the file, by their place in its drawn diff, picked for a comment. */
  pick:
    | {
        from: number;
        to: number;
      }
    | undefined;
  marks: Readonly<Record<string, FindingMark>>;
  /** The key list is up. */
  keys: boolean;
  wanted: Wanted | undefined;
};

/** Go to file, while it is up, and where a pick goes: Review, or Files where it was opened from Files. */
export type GoTo = {
  key: string;
  to: "review" | "files";
};

const EMPTY: ReviewTabState = {
  path: undefined,
  stop: undefined,
  whole: false,
  finding: undefined,
  pick: undefined,
  marks: {},
  keys: false,
  wanted: undefined,
};

const states = new Map<string, ReviewTabState>();
let goTo: GoTo | undefined;
let asked = 0;
const listeners = new Set<() => void>();
const told = () => listeners.forEach((one) => one());
const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => listeners.delete(listener);
};

export const reviewTabOf = (key: string): ReviewTabState => states.get(key) ?? EMPTY;

function change(key: string, to: Partial<ReviewTabState>): void {
  states.set(key, {
    ...reviewTabOf(key),
    ...to,
  });
  told();
}

export const useReviewTabState = (key: string): ReviewTabState =>
  useSyncExternalStore(subscribe, () => reviewTabOf(key));

export const useGoTo = (): GoTo | undefined => useSyncExternalStore(subscribe, () => goTo);

export const reviewTab = {
  change,
  /** Reads `path`, at a finding or a line of it where one is named, and brings that into view. */
  go(
    key: string,
    path: string,
    at: {
      finding?: string;
      line?: {
        line: number;
        side: DiffSide;
      };
    } = {},
  ): void {
    asked += 1;
    change(key, {
      path,
      stop: undefined,
      whole: false,
      finding: at.finding,
      pick: undefined,
      wanted: {
        n: asked,
        ...at,
      },
    });
  },
  /** Goes to a stop of the tour, its lines only, at a finding of it where one is named. */
  toStop(key: string, stop: number, finding?: string): void {
    asked += 1;
    change(key, {
      path: undefined,
      stop,
      whole: false,
      finding,
      pick: undefined,
      wanted: {
        n: asked,
        ...(finding === undefined
          ? {}
          : {
              finding,
            }),
      },
    });
  },
  mark(key: string, finding: string, mark: FindingMark | undefined): void {
    const { [finding]: _, ...rest } = reviewTabOf(key).marks;
    change(key, {
      marks:
        mark === undefined
          ? rest
          : {
              ...rest,
              [finding]: mark,
            },
    });
  },
  openGoTo(key: string, to: GoTo["to"]): void {
    goTo = {
      key,
      to,
    };
    told();
  },
  closeGoTo(): void {
    goTo = undefined;
    told();
  },
};

/** The lines of `lines` a finding is about, by their place in the drawn diff. None where the diff does not hold them. */
export function linesOfFinding(lines: readonly DiffLine[], finding: Finding): number[] {
  const side = finding.at.side === "right" ? "now" : "old";
  return counted(lines).flatMap((one, at) => {
    const number = one[side];
    return number !== undefined && number >= finding.at.start && number <= finding.at.end
      ? [
          at,
        ]
      : [];
  });
}

/**
 * The files Review steps through, the ones that need him: those with findings first, then those with open threads, each in the
 * order the pull request lists them. The rest are reached from Go to file and Files. The findings go file by file, top to bottom.
 */
export function orderOf(
  pr: ReviewPr,
  work: Pick<PrWork, "findings" | "threads">,
): {
  files: string[];
  findings: Finding[];
} {
  const findings = work.findings.filter((one) => !one.dismissed);
  const listed = pr.page.files.map((one) => one.path);
  const found = new Set(findings.map((one) => one.at.path));
  const talked = new Set(work.threads.filter((one) => !one.resolved).map((one) => one.path));
  const files = [
    ...listed.filter((one) => found.has(one)),
    ...listed.filter((one) => !found.has(one) && talked.has(one)),
  ];
  return {
    files,
    findings: [
      ...findings,
    ].sort(
      (a, b) =>
        files.indexOf(a.at.path) - files.indexOf(b.at.path) ||
        a.at.start - b.at.start ||
        a.id.localeCompare(b.id),
    ),
  };
}

/** The file Review reads: the one asked for, which Go to file may take outside `files`, else the first of `files`. */
export const pathOf = (
  pr: ReviewPr,
  asked: string | undefined,
  files: readonly string[],
): string | undefined =>
  asked !== undefined && pr.page.files.some((one) => one.path === asked) ? asked : files[0];

/** A stop Review steps to: one of the tour's, or one it adds at the end for what no stop of the tour holds. */
export type Stop = {
  title: string;
  path: string;
  /** The lines it is about. Absent: the whole file. */
  place: ReviewPlace | undefined;
  /** The findings it holds, by id, top to bottom. */
  findings: readonly string[];
  /** The threads it holds, by id, resolved ones too. */
  threads: readonly string[];
};

/** Lines `start` to `end` on `side` are at a stop: inside its lines or within the lines drawn around them. */
const near = (
  stop: Pick<Stop, "path" | "place">,
  path: string,
  side: DiffSide,
  start: number,
  end: number,
): boolean =>
  stop.path === path &&
  (stop.place === undefined ||
    (stop.place.side === side &&
      start <= stop.place.end + CONTEXT &&
      end >= stop.place.start - CONTEXT));

/**
 * The stops Review steps through once Tour has run: the tour's own in its order, each holding the findings and threads on its lines,
 * then one for each file whose findings or open threads no stop holds, titled `Also in` its path. A thread on a whole file is held by
 * the first stop on that file. None where Tour has not come to any stop, and Review steps by file.
 */
export function stopsOf(
  pr: ReviewPr,
  work: Pick<PrWork, "findings" | "threads" | "wire">,
): Stop[] | undefined {
  const tour = tourOf(work.wire[TOUR_RUN]);
  const listed = pr.page.files.map((one) => one.path);
  const toured = (tour ?? []).filter((one) => listed.includes(one.at.path));
  if (toured.length === 0) return undefined;
  const findings = work.findings
    .filter((one) => !one.dismissed)
    .sort((a, b) => a.at.start - b.at.start || a.id.localeCompare(b.id));
  const firstOn = (path: string) => toured.findIndex((one) => one.at.path === path);
  const holds = (
    stop: Pick<Stop, "path" | "place">,
    at: number,
    thread: PrWork["threads"][number],
  ): boolean =>
    thread.line === undefined
      ? stop.place === undefined
        ? stop.path === thread.path
        : firstOn(thread.path) === at
      : near(stop, thread.path, thread.side, thread.line, thread.line);
  const held = (stop: Pick<Stop, "path" | "place">, at: number) => ({
    findings: findings
      .filter((one) => near(stop, one.at.path, one.at.side, one.at.start, one.at.end))
      .map((one) => one.id),
    threads: work.threads.filter((one) => holds(stop, at, one)).map((one) => one.id),
  });
  const stops: Stop[] = toured.map((one, at) => {
    const stop = {
      title: one.title,
      path: one.at.path,
      place: one.at,
    };
    return {
      ...stop,
      ...held(stop, at),
    };
  });
  const found = new Set(stops.flatMap((one) => one.findings));
  const talked = new Set(stops.flatMap((one) => one.threads));
  const leftFindings = findings.filter((one) => !found.has(one.id));
  const leftThreads = work.threads.filter((one) => !one.resolved && !talked.has(one.id));
  for (const path of listed) {
    const left = [
      ...leftFindings
        .filter((one) => one.at.path === path)
        .map((one) => ({
          side: one.at.side,
          start: one.at.start as number | undefined,
          end: one.at.end as number | undefined,
        })),
      ...leftThreads
        .filter((one) => one.path === path)
        .map((one) => ({
          side: one.side,
          start: one.line,
          end: one.line,
        })),
    ];
    if (left.length === 0) continue;
    // The lines around what is left where they are all lines of one side, else the whole file.
    const side = left[0]!.side;
    const lined = left.every((one) => one.start !== undefined && one.side === side);
    stops.push({
      title: `Also in ${path}`,
      path,
      place: lined
        ? {
            path,
            side,
            start: Math.min(...left.map((one) => one.start!)),
            end: Math.max(...left.map((one) => one.end!)),
          }
        : undefined,
      findings: leftFindings.filter((one) => one.at.path === path).map((one) => one.id),
      threads: leftThreads.filter((one) => one.path === path).map((one) => one.id),
    });
  }
  return stops;
}

/**
 * Where Review is on a tour: the stop it shows, or -1 on a file no stop is on, and whether the whole file is drawn. A file asked for
 * from Go to file, a peek's Jump or a draft is at the stop holding the finding or line it was asked at, else drawn whole at its first stop.
 */
export function stopOf(
  state: Pick<ReviewTabState, "path" | "stop" | "whole" | "finding" | "wanted">,
  stops: readonly Stop[],
): {
  at: number;
  whole: boolean;
} {
  const path = state.path;
  if (path === undefined)
    return {
      at: Math.max(0, Math.min(state.stop ?? 0, stops.length - 1)),
      whole: state.whole,
    };
  const line = state.wanted?.line;
  const exact = stops.findIndex(
    (one) =>
      one.path === path &&
      ((state.finding !== undefined && one.findings.includes(state.finding)) ||
        (line !== undefined && near(one, path, line.side, line.line, line.line))),
  );
  if (exact >= 0)
    return {
      at: exact,
      whole: state.whole,
    };
  return {
    at: stops.findIndex((one) => one.path === path),
    whole: true,
  };
}

/** The finding after (or before) the one in focus, going on into the next file's. Absent at either end. */
export function stepFinding(
  findings: readonly Finding[],
  from: string | undefined,
  path: string | undefined,
  by: 1 | -1,
): Finding | undefined {
  const at = findings.findIndex((one) => one.id === from);
  if (at >= 0) return findings[at + by];
  // None in focus: the first of the file he is reading, or of the next file on.
  const here = findings.findIndex((one) => one.at.path === path);
  return by === 1 ? findings[here < 0 ? 0 : here] : undefined;
}

/** The finding after (or before) the one in focus on a tour, stop by stop, and the stop it is at. Absent at either end. */
function stepTour(
  stops: readonly Stop[],
  from: string | undefined,
  at: number,
  by: 1 | -1,
):
  | {
      stop: number;
      finding: string;
    }
  | undefined {
  const ids = [
    ...new Set(stops.flatMap((one) => one.findings)),
  ];
  const was = from === undefined ? -1 : ids.indexOf(from);
  // None in focus: the first of the stop he is at, or of the next stop on.
  const to =
    was >= 0
      ? ids[was + by]
      : by === 1
        ? stops.slice(Math.max(0, at)).flatMap((one) => one.findings)[0]
        : undefined;
  if (to === undefined) return undefined;
  return {
    stop: stops[at]?.findings.includes(to)
      ? at
      : stops.findIndex((one) => one.findings.includes(to)),
    finding: to,
  };
}

/**
 * A key pressed while the Review tab has the keyboard: `j` and `k` step through the findings and on into the next file's, or the next
 * stop's on a tour; `]` and `[` go to the next and previous file, or stop; `?` puts up the key list, and Esc takes down the key list or
 * the picked lines. Answers whether it took the key; an Esc with nothing to close goes on to back out a level, as everywhere in the review.
 */
export function reviewTabKey(
  event: Pick<KeyboardEvent, "key">,
  pr: ReviewPr,
  work: Pick<PrWork, "findings" | "threads" | "wire">,
): boolean {
  const state = reviewTabOf(pr.key);
  const stops = stopsOf(pr, work);
  if (
    stops !== undefined &&
    [
      "j",
      "k",
      "]",
      "[",
    ].includes(event.key)
  ) {
    const { at } = stopOf(state, stops);
    const by = event.key === "j" || event.key === "]" ? 1 : -1;
    if (event.key === "j" || event.key === "k") {
      const to = stepTour(stops, state.finding, at, by);
      if (to !== undefined) reviewTab.toStop(pr.key, to.stop, to.finding);
    } else if (stops[at + by] !== undefined) reviewTab.toStop(pr.key, at + by);
    return true;
  }
  const { files, findings } = orderOf(pr, work);
  const path = pathOf(pr, state.path, files);
  switch (event.key) {
    case "j":
    case "k": {
      const to = stepFinding(findings, state.finding, path, event.key === "j" ? 1 : -1);
      if (to !== undefined)
        reviewTab.go(pr.key, to.at.path, {
          finding: to.id,
        });
      return true;
    }
    case "]":
    case "[": {
      const to =
        files[(path === undefined ? -1 : files.indexOf(path)) + (event.key === "]" ? 1 : -1)];
      if (to !== undefined) reviewTab.go(pr.key, to);
      return true;
    }
    case "?":
      change(pr.key, {
        keys: !state.keys,
      });
      return true;
    case "Escape":
      if (state.keys) {
        change(pr.key, {
          keys: false,
        });
        return true;
      }
      if (state.pick !== undefined) {
        change(pr.key, {
          pick: undefined,
        });
        return true;
      }
      return false;
    default:
      return false;
  }
}
