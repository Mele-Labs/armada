// The Job `mute-fleet-notice` and the mute test open: its thread holds Fleet's behind-base notice and another Job's message.

import type { SessionRow } from "@armada/protocol";

import { held, hosted } from "../sessions-fleet";

export const NOTICE_JOB = "01SESSIONAAAAAAAAAAAAAAAAA";
const AT = "2026-10-15T13:31:00.000Z";

export const noticeSessions = () => [hosted(NOTICE_JOB, { title: "Fix the flaky store test", attachments: [held("slot", "4"), held("branch", "armada/session-2fabbcd9")] })];

export const noticeThreads = (): Record<string, SessionRow[]> => ({
  [NOTICE_JOB]: [
    { kind: "message", id: "m1", at: AT, from: { kind: "you" }, text: "Pin the clock in the store tests." },
    {
      kind: "message",
      id: "m2",
      at: AT,
      from: { kind: "session", id: "fleet", title: "Fleet" },
      text: "slot-4 (branch armada/session-2fabbcd9) is 23 commits behind master. Merge master into your branch before your next push.",
      notice: "behind_base",
    },
    { kind: "message", id: "m3", at: AT, from: { kind: "session", id: "other", title: "Round the totals" }, text: "Your branch touches the totals file I am in." },
  ],
});
