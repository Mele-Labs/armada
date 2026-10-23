// Which row the rail marks. Overview is where a window with nothing else open
// is, so everything else — the reports — is Overview with something over it.
// Apart from `App.tsx`, which is at its length.

import { SURFACE } from "@armada/shell";

/** Which surface is open, as `App` holds it: one flag each, at most one set. */
export type Open = {
  clearing: boolean;
  manifesting: boolean;
  settingsShowing: boolean;
  modding: boolean;
  kitting: boolean;
  guiding: boolean;
  studying: boolean;
  lining: boolean;
  learning: boolean;
  workflowing: boolean;
  checking: boolean;
  /** Doctor, reached from the Fleet panel's Doctor row. Optional so a caller that never opens it need not say so. */
  doctoring?: boolean;
  /** The account's page, reached from the title row's menu and from Settings. */
  accounting?: boolean;
  /** Reviews: the pull requests waiting on him. */
  reviewing?: boolean;
  /** Issues: a repository's tracker. */
  issuing?: boolean;
  /** Usage: where the tokens went. */
  usaging?: boolean;
};

/** The rail row for what is open, in the precedence `App` draws by. */
export function showingOf(open: Open): string {
  if (open.clearing) return SURFACE.worktrees;
  if (open.manifesting) return SURFACE.manifest;
  if (open.settingsShowing) return SURFACE.settings;
  if (open.accounting === true) return SURFACE.account;
  if (open.modding) return SURFACE.mods;
  if (open.kitting) return SURFACE.kit;
  if (open.guiding) return SURFACE.guides;
  if (open.studying) return SURFACE.studios;
  if (open.lining) return SURFACE.mergeLine;
  if (open.learning) return SURFACE.lessons;
  if (open.workflowing) return SURFACE.workflows;
  if (open.checking) return SURFACE.checks;
  if (open.reviewing === true) return SURFACE.reviews;
  if (open.issuing === true) return SURFACE.issues;
  if (open.usaging === true) return SURFACE.usage;
  if (open.doctoring === true) return SURFACE.doctor;
  return SURFACE.overview;
}
