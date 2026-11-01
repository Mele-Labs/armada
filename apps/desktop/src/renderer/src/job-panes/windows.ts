// Moving a tab out into a window of its own. Each kind that can leave has a window main already draws: the Job browser's and the Job's shell's.

import { browserToWindow } from "../job-browser/store";
import { shellsOf } from "../job-shell";
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
export function moveToWindow(jobId: string, tab: Tab): void {
  switch (tab.kind) {
    case "browser":
      browserToWindow(jobId, tab.id);
      return;
    case "terminal": {
      // One place at a time: the pane puts its screen away, so the window's is the one that sizes the shell.
      const shells = shellsOf(window.armada);
      if (shells?.toWindow(jobId)) shells.show("page", jobId, false);
      return;
    }
    default:
      return;
  }
}
