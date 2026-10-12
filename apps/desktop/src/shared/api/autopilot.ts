// Autopilot: a repository works its own backlog from one sentence until the owner pauses or stops it.
// #2092, `docs/concepts/autopilot.md`. A slice imports protocol, never another slice; `../api.ts` and
// `../bridge.ts` compose them.

import type { Autopilot, AutopilotList, AutopilotProposal, AutopilotSettings, Outcome } from "@armada/protocol";

/** What an Autopilot act came to: Fleet's answer, or the refusal to word. */
export type AutopilotActed<T = Autopilot> = { ok: true; value: T } | { ok: false; outcome: Outcome };

export type AutopilotApi = {
  /** One sentence read as settings, with the words each came from, for the person to confirm or change. **Starts nothing.** */
  proposeAutopilot: (manifestId: string, sentence: string, localNow: string) => Promise<AutopilotActed<AutopilotProposal>>;
  /** Start with what the person confirmed. */
  startAutopilot: (manifestId: string, settings: AutopilotSettings) => Promise<AutopilotActed>;
  /** New settings for one running or paused, in its state: never a resume. Raising the cap that paused it lets Resume through. */
  updateAutopilot: (manifestId: string, settings: AutopilotSettings) => Promise<AutopilotActed>;
  /** No new picks; the Jobs it started finish. */
  pauseAutopilot: (manifestId: string) => Promise<AutopilotActed>;
  /** Refused (409 `fleet.autopilot_capped`) while a cap it reached still holds, naming the cap. */
  resumeAutopilot: (manifestId: string) => Promise<AutopilotActed>;
  /** No new picks, what it picked and has not started withdrawn, and the end. */
  stopAutopilot: (manifestId: string) => Promise<AutopilotActed>;
  listAutopilots: () => Promise<AutopilotActed<AutopilotList>>;
  getAutopilot: (manifestId: string) => Promise<AutopilotActed>;
  /** A set-aside pick, dispatched by the person: it waits at the approval gate. */
  dispatchSetAside: (manifestId: string, pickId: string) => Promise<AutopilotActed>;
  dismissSetAside: (manifestId: string, pickId: string) => Promise<AutopilotActed>;
  /** `autopilot.changed`: one repository's Autopilot, whole, whenever Fleet changes it. */
  onAutopilotChanged: (onChanged: (autopilot: Autopilot) => void) => () => void;
};

export const AUTOPILOT_CHANNELS = {
  proposeAutopilot: "bridge:propose-autopilot",
  startAutopilot: "bridge:start-autopilot",
  updateAutopilot: "bridge:update-autopilot",
  pauseAutopilot: "bridge:pause-autopilot",
  resumeAutopilot: "bridge:resume-autopilot",
  stopAutopilot: "bridge:stop-autopilot",
  listAutopilots: "bridge:list-autopilots",
  getAutopilot: "bridge:get-autopilot",
  dispatchSetAside: "bridge:dispatch-set-aside",
  dismissSetAside: "bridge:dismiss-set-aside",
  /** Main to every window, not a request. */
  autopilotChanged: "bridge:autopilot-changed",
} as const;
