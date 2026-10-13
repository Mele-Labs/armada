// The weekly window nearing its end, and the default model offered a step down until it resets. The title bar
// holds the plan's limits at 5h 30% and wk 75%, and one live Session shows the default in its model list. A walk's
// `later` moves the week to 83% (the offer), then to a new window (the restore). The default is `models.default`
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

/** Each window's end is read once, so the week is one window until the walk says it is over. */
const FIVE = fromNow(2, 14);
const WEEK = fromNow(2 * 24 + 17, 0);
const NEXT_WEEK = new Date(new Date(WEEK).getTime() + 7 * 24 * 60 * 60_000).toISOString();

const reading = (five: number, week: number, weekEnds: string): PlanUsage => ({
  windows: [
    { kind: "five_hour", percent_used: five, resets_at: FIVE },
    { kind: "seven_day", percent_used: week, resets_at: weekEnds },
  ],
  read_at: now(),
});

/** What Fleet reads as the walk goes on: under the line, over it, then a new week. */
const READINGS = [() => reading(30, 75, WEEK), () => reading(41, 83, WEEK), () => reading(6, 3, NEXT_WEEK)];

function build(): Scenario {
  const board = onBoard([], { repositories: [ARMADA], picked: ARMADA.root });
  const served = new FakeSessionsFleet(records(), {}).scenario(board);
  const models = served.state.holds.models;
  return {
    ...served,
    name: "plan-downshift",
    says: "The title bar holding the weekly limit at 75%, which the walk takes over 80% and then past its reset; the Session's model list shows the default",
    state: { ...served.state, holds: { ...served.state.holds, models: { models: ["haiku", "sonnet", "opus"], ...models, default: "opus" } } },
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
        // Fleet answers a save with the settings now in force, and the model list it serves follows `models.default`.
        saveSettings: async (changes) => {
          const held = handle.state().settings;
          if (held !== null) handle.publish({ settings: savedInto(held, changes) });
          const { holds, settings } = handle.state();
          const chosen = "models.default" in changes ? settings?.settings.find((one) => one.key === "models.default")?.value : undefined;
          if (holds.models !== null && typeof chosen === "string") handle.publish({ holds: { ...holds, models: { ...holds.models, default: chosen } } });
          return { ok: true };
        },
      };
    },
  };
}

export const planDownshift: Scenario = build();
