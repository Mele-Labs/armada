// Which row the rail marks. Overview is where a window with nothing else open
// is, so everything else — a Job, the composer, the reports — is Overview with
// something over it. That sentence was the Board's until the owner deleted
// that page. Apart from `App.tsx`, which is at its length.

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
  sessioning: boolean;
};

/** The rail row for what is open, in the precedence `App` draws by. */
export function showingOf(open: Open): string {
  if (open.clearing) return SURFACE.worktrees;
  if (open.manifesting) return SURFACE.manifest;
  if (open.settingsShowing) return SURFACE.settings;
  if (open.modding) return SURFACE.mods;
  if (open.kitting) return SURFACE.kit;
  if (open.guiding) return SURFACE.guides;
  if (open.studying) return SURFACE.studios;
  if (open.lining) return SURFACE.mergeLine;
  if (open.learning) return SURFACE.lessons;
  if (open.workflowing) return SURFACE.workflows;
  if (open.checking) return SURFACE.checks;
  if (open.sessioning) return SURFACE.sessions;
  return SURFACE.overview;
}
