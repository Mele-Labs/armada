// The Logbook page's reads and acts. A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts`
// compose them. `docs/concepts/logbook.md`.
//
// Main answers each channel from Fleet's routes (`main/logbook.ts`); the mock answers them in a walk.

import type { LogbookAnswer, LogbookForgotten, LogbookOrganizeApplied, LogbookOrganized, LogbookRead, LogbookTidied, OrganizeAction, OrganizeProposal } from "@armada/protocol";

export type LogbookApi = {
  /** Every section: each repository's, Habits and This Mac. Forgotten entries are never read. */
  readLogbook: () => Promise<LogbookRead>;
  /** Replaces the entry's text and marks it edited. */
  editLogbookEntry: (id: string, text: string) => Promise<LogbookAnswer>;
  /** A locked entry never fades and is never rewritten. */
  lockLogbookEntry: (id: string, locked: boolean) => Promise<LogbookAnswer>;
  /** Puts the entry on the deny list, so no later Job brings it back. */
  forgetLogbookEntry: (id: string) => Promise<LogbookForgotten>;
  /** Starts folding near-duplicate entries across every repository and answers at once; the Logbook read says when it is done. */
  tidyLogbook: () => Promise<LogbookTidied>;
  /** What an action proposes across every repository; nothing changes. Merge and bugs wait on a model call. */
  organizeLogbook: (action: OrganizeAction) => Promise<LogbookOrganized>;
  /** Carries out the proposals a person kept, as Organize answered them. */
  applyOrganizeLogbook: (action: OrganizeAction, proposals: OrganizeProposal[]) => Promise<LogbookOrganizeApplied>;
};

export const LOGBOOK_CHANNELS = {
  readLogbook: "bridge:read-logbook",
  editLogbookEntry: "bridge:edit-logbook-entry",
  lockLogbookEntry: "bridge:lock-logbook-entry",
  forgetLogbookEntry: "bridge:forget-logbook-entry",
  tidyLogbook: "bridge:tidy-logbook",
  organizeLogbook: "bridge:organize-logbook",
  applyOrganizeLogbook: "bridge:apply-organize-logbook",
} as const;
