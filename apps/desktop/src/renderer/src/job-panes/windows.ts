// Moving a tab out into a window of its own. Each kind that can leave has a window main already draws: the Job browser's and the Job's shell's.

import { KINDS } from "./kinds";
import type { Tab, TabKind } from "./tree";

/** Why a kind cannot leave the window, in plain words; absent where it can. */
export function whyNotWindow(kind: TabKind): string | undefined {
  switch (kind) {
    case "browser":
    case "terminal":
      return undefined;
    default:
      return `${KINDS[kind].label} has no window of its own`;
  }
}

/** The tab's page or shell opens in its own window. */
export function moveToWindow(_jobId: string, tab: Tab): void {
  switch (tab.kind) {
    case "browser":
      // Filled by the Job browser's per-tab pages.
      return;
    case "terminal":
      // Filled by the shell's window path.
      return;
    default:
      return;
  }
}
