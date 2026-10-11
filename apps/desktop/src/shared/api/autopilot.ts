// Autopilot: a repository works its own backlog from one sentence until the owner pauses or stops it.
// #2092, `docs/concepts/autopilot.md`. A slice imports protocol, never another slice; `../api.ts` and
// `../bridge.ts` compose them.

import type { Autopilot, AutopilotList, AutopilotSettings, Outcome } from "@armada/protocol";

/** What an Autopilot act came to: Fleet's answer, or the refusal to word. */
export type AutopilotActed<T = Autopilot> = { ok: true; value: T } | { ok: false; outcome: Outcome };

export type AutopilotApi = {
  /** One sentence read as settings, for the person to confirm or change. **Starts nothing.** */
  proposeAutopilot: (manifestId: string, sentence: string, localNow: string) => Promise<AutopilotActed<AutopilotSettings>>;
  /** Start with what the person confirmed. */
  startAutopilot: (manifestId: string, settings: AutopilotSettings) => Promise<AutopilotActed>;
  /** No new picks; the Jobs it started finish. */
  pauseAutopilot: (manifestId: string) => Promise<AutopilotActed>;
  resumeAutopilot: (manifestId: string) => Promise<AutopilotActed>;
  /** No new picks, what it picked and has not started withdrawn, and the end. */
  stopAutopilot: (manifestId: string) => Promise<AutopilotActed>;
  listAutopilots: () => Promise<AutopilotActed<AutopilotList>>;
  getAutopilot: (manifestId: string) => Promise<AutopilotActed>;
  /** A set-aside pick, dispatched by the person: it waits at the approval gate. */
  dispatchSetAside: (manifestId: string, pickId: string) => Promise<AutopilotActed>;
  dismissSetAside: (manifestId: string, pickId: string) => Promise<AutopilotActed>;
};

export const AUTOPILOT_CHANNELS = {
  proposeAutopilot: "bridge:propose-autopilot",
  startAutopilot: "bridge:start-autopilot",
  pauseAutopilot: "bridge:pause-autopilot",
  resumeAutopilot: "bridge:resume-autopilot",
  stopAutopilot: "bridge:stop-autopilot",
  listAutopilots: "bridge:list-autopilots",
  getAutopilot: "bridge:get-autopilot",
  dispatchSetAside: "bridge:dispatch-set-aside",
  dismissSetAside: "bridge:dismiss-set-aside",
} as const;
