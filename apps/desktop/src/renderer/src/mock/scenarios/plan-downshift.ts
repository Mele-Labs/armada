// The weekly window nearing its end, and the default model offered a step down until it resets. The title bar
// holds the plan's limits at 5h 30% and wk 75%, and one live Session shows the default in its model list. A walk's
// `later` moves the week to 83% (the offer), then to a new window (the restore). The Session default is `models.session`
// in settings.json, so pressing the offer and the Settings page agree. The walk `planDownshift` plays it.

import type { PlanUsage, RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";
import { savedInto } from "@armada/settings/fake";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, hosted } from "../sessions-fleet";
import { onTimePassing } from "../time-passes";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const now = () => new Date().toISOString();
const fromNow = (hours: number, minutes: number): string => new Date(Date.now() + (hours * 60 + minutes) * 60_000 + 30_000).toISOString();

const records = () => [hosted("01DSHIFTAAAAAAAAAAAAAAAAAA", { title: "Cart totals rounding", last_turn_at: now() })];

/** Each window's end is fixed once read, so the week stays one window until the walk says it is over. */
const ends = { five: fromNow(2, 14), week: fromNow(2 * 24 + 17, 0), then: { five: "", week: "" } };

const reading = (five: number, week: number, over = false): PlanUsage => {
  // The next windows start when the walk reaches them, so they end a full window from then.
  if (over && ends.then.week === "") ends.then = { five: fromNow(5, 0), week: fromNow(7 * 24, 0) };
  return {
    windows: [
      { kind: "five_hour", percent_used: five, resets_at: over ? ends.then.five : ends.five },
      { kind: "seven_day", percent_used: week, resets_at: over ? ends.then.week : ends.week },
    ],
    read_at: now(),
  };
};

/** What Fleet reads as the walk goes on: under the line, over it, then a new week. */
const READINGS = [() => reading(30, 75), () => reading(41, 83), () => reading(6, 3, true)];

function build(): Scenario {
  const board = onBoard([], { repositories: [ARMADA], picked: ARMADA.root });
  const served = new FakeSessionsFleet(records(), {}).scenario(board);
  const models = served.state.holds.models;
  return {
    ...served,
    name: "plan-downshift",
    says: "The title bar holding the weekly limit at 75%, which the walk takes over 80% and then past its reset; the Session's model list shows the default",
    state: { ...served.state, holds: { ...served.state.holds, models: { models: ["haiku", "sonnet", "opus"], default: "sonnet", ...models, session: "opus" } } },
    behaves: (handle) => {
      const fleet = new FakeSessionsFleet(records(), {}).scenario(board).behaves!(handle);
      let at = 0;
      const heard = new Set<(usage: PlanUsage) => void>();
      // After the window has registered its own hook, and the latest wins: a walk's `later` is the next reading.
      queueMicrotask(() =>
        onTimePassing(() => {
          at = Math.min(at + 1, READINGS.length - 1);
          const next = READINGS[at]!();
          heard.forEach((on) => on(next));
        }),
      );
      return {
        ...fleet,
        getPlanUsage: async () => ({ ok: true, value: READINGS[at]!() }),
        onPlanUsageChanged: (on) => {
          heard.add(on);
          return () => void heard.delete(on);
        },
        // Fleet answers a save with the settings now in force, and the model list it serves follows `models.session`.
        saveSettings: async (changes) => {
          const held = handle.state().settings;
          if (held !== null) handle.publish({ settings: savedInto(held, changes) });
          const { holds, settings } = handle.state();
          const chosen = "models.session" in changes ? settings?.settings.find((one) => one.key === "models.session")?.value : undefined;
          if (holds.models !== null && typeof chosen === "string") handle.publish({ holds: { ...holds, models: { ...holds.models, session: chosen } } });
          return { ok: true };
        },
      };
    },
  };
}

export const sessionPlanDownshift: Scenario = build();
