// The Session whose Check is still out, opened on its log: the lines grow while it runs, and the
// panel's head ends it, gently or by force. Over `session-checks`. The walk `session-check-live-log`
// plays it, and `session-check-live-log.test.tsx` holds that each button reaches its route.

import type { Scenario } from "../moment";
import { s203SessionChecks } from "./session-checks";

/** Each stop the panel asked, for the test to read. Emptied as the scenario starts. */
export const stopsAsked: { run: number; force: boolean }[] = [];

const FIRST = ["$ vitest run --project app_smoke", " RUN  v3.2.4"];
const ARRIVING = [" ✓ src/renderer/src/mock/walks.test.tsx (12 tests)", " ✓ src/renderer/src/mock/sessions.test.tsx (4 tests)"];
const EVERY_MS = 700;

export const s208SessionCheckLiveLog: Scenario = {
  ...s203SessionChecks,
  name: "session-check-live-log",
  says: "A Session's Check is still out, and its log grows in the panel while it runs, with Stop and Force stop in the head",
  behaves: (fleet) => {
    stopsAsked.length = 0;
    let timers: ReturnType<typeof setTimeout>[] = [];
    const show = (jobId: string, kept: string, lines: string[]) =>
      fleet.publish({ followed: { state: "following", jobId, kept, name: "app_smoke", attempt: 1, path: "session-check/31", fromLine: 1, lines } });
    return {
      ...s203SessionChecks.behaves?.(fleet),
      followCheckOutput: async (jobId, kept) => {
        timers.forEach(clearTimeout);
        timers = [];
        if (jobId === null || kept === null) {
          fleet.publish({ followed: { state: "none" } });
          return;
        }
        show(jobId, kept, FIRST);
        ARRIVING.forEach((_, n) => {
          timers.push(setTimeout(() => show(jobId, kept, [...FIRST, ...ARRIVING.slice(0, n + 1)]), EVERY_MS * (n + 1)));
        });
      },
      stopSessionCheck: async (run, force) => {
        stopsAsked.push({ run, force });
        return { ok: true };
      },
    };
  },
};
