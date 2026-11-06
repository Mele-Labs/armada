// What Focus holds for each open pull request, kept outside the tab so a key, a peek's Jump and Go to file can move it from
// anywhere: the file it reads, the finding in focus, the lines picked for a comment and how each finding was marked. Kept for the
// window's life, not on disk: a mark is his reading of one run, and nothing on the forge sees it.

import { useSyncExternalStore } from "react";
import type { DiffSide } from "@armada/protocol";

import { counted } from "./patch";
import type { DiffLine } from "@armada/components";
import type { Finding, PrWork, ReviewPr } from "./model";

/** How he marked a finding. Any mark puts it away. */
export type FindingMark = "expected" | "not-valid" | "commented";

/** What Focus is asked to bring into view next. A new `n` each ask, so asking for the same place twice moves there twice. */
export type Wanted = {
  n: number;
  finding?: string;
  line?: {
    line: number;
    side: DiffSide;
  };
};

export type FocusState = {
  /** The file Focus reads. Absent: the first in its order. */
  path: string | undefined;
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

/** Go to file, while it is up, and where a pick goes: Focus, or Files where it was opened from Files. */
export type GoTo = {
  key: string;
  to: "focus" | "files";
};

const EMPTY: FocusState = {
  path: undefined,
  finding: undefined,
  pick: undefined,
  marks: {},
  keys: false,
  wanted: undefined,
};

const states = new Map<string, FocusState>();
let goTo: GoTo | undefined;
let asked = 0;
const listeners = new Set<() => void>();
const told = () => listeners.forEach((one) => one());
const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => listeners.delete(listener);
};

export const focusOf = (key: string): FocusState => states.get(key) ?? EMPTY;

function change(key: string, to: Partial<FocusState>): void {
  states.set(key, {
    ...focusOf(key),
    ...to,
  });
  told();
}

export const useFocus = (key: string): FocusState =>
  useSyncExternalStore(subscribe, () => focusOf(key));

export const useGoTo = (): GoTo | undefined => useSyncExternalStore(subscribe, () => goTo);

export const focus = {
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
      finding: at.finding,
      pick: undefined,
      wanted: {
        n: asked,
        ...at,
      },
    });
  },
  mark(key: string, finding: string, mark: FindingMark | undefined): void {
    const { [finding]: _, ...rest } = focusOf(key).marks;
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
 * The order Focus reads in: files with findings first, then the rest, each in the order the pull request lists them, and the
 * findings file by file, top to bottom.
 */
export function orderOf(
  pr: ReviewPr,
  work: Pick<PrWork, "findings">,
): {
  files: string[];
  findings: Finding[];
} {
  const findings = work.findings.filter((one) => !one.dismissed);
  const listed = pr.page.files.map((one) => one.path);
  const found = new Set(findings.map((one) => one.at.path));
  const files = [
    ...listed.filter((one) => found.has(one)),
    ...listed.filter((one) => !found.has(one)),
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

/**
 * A key pressed while Focus has the keyboard: `j` and `k` step through the findings and on into the next file's, `]` and `[` go
 * to the next and previous file, `?` puts up the key list, and Esc takes down the key list or the picked lines. Answers whether
 * it took the key; an Esc with nothing to close goes on to back out a level, as everywhere in the review.
 */
export function focusKey(
  event: Pick<KeyboardEvent, "key">,
  pr: ReviewPr,
  work: Pick<PrWork, "findings">,
): boolean {
  const state = focusOf(pr.key);
  const { files, findings } = orderOf(pr, work);
  const path = state.path !== undefined && files.includes(state.path) ? state.path : files[0];
  switch (event.key) {
    case "j":
    case "k": {
      const to = stepFinding(findings, state.finding, path, event.key === "j" ? 1 : -1);
      if (to !== undefined)
        focus.go(pr.key, to.at.path, {
          finding: to.id,
        });
      return true;
    }
    case "]":
    case "[": {
      const to =
        files[(path === undefined ? -1 : files.indexOf(path)) + (event.key === "]" ? 1 : -1)];
      if (to !== undefined) focus.go(pr.key, to);
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
