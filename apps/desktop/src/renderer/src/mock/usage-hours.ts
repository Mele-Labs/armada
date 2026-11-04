// The last 48 hours, an hour a point, for the Usage page's two line charts. Mock only: a
// sketch, put on the wire by `usage-time-wire.ts`, shaped to tell one story. Burn swings through the working day and stops overnight, so those hours are gaps. The
// model's wait, a tool call and a pull request's wait hold; Checks per run steps up by about 40% at the preview `7ac3d52`
// and stays up. Hours with no pull request are gaps in that line.

import type { UsageHour, UsageHours } from "@armada/components";

import { usageBuildsNow } from "./usage-builds";

const HOUR = 3_600_000;
const SHOWN = 48;

/** The share of a full hour's burn by hour of the local day: nothing from midnight to six. */
const PROFILE = [
  0, 0, 0, 0, 0, 0, 0.15, 0.4, 0.7, 1, 0.9, 0.8, 0.5, 0.85, 1, 0.95, 0.8, 0.6, 0.35, 0.5, 0.7, 0.6, 0.35, 0.15,
];
const FULL_HOUR = 21;

/** A steady pseudo-random number in 0 to 1 for an hour and a purpose, so the same hour reads the same each time. */
const wobble = (hour: number, salt: number): number => {
  const x = Math.sin(hour * 12.9898 + salt * 78.233) * 43758.5453;
  return x - Math.floor(x);
};

export function usageHoursNow(now = Date.now()): UsageHours {
  const last = Math.floor(now / HOUR) * HOUR;
  const step = usageBuildsNow(now).find((one) => one.id === "7ac3d52");
  const stepAt = step === undefined ? Number.POSITIVE_INFINITY : Date.parse(step.started);
  return Array.from(
    {
      length: SHOWN,
    },
    (_, at): UsageHour => {
      const start = last - (SHOWN - 1 - at) * HOUR;
      const key = Math.round(start / HOUR);
      const share = PROFILE[new Date(start).getHours()]!;
      const begins = new Date(start).toISOString();
      if (share === 0)
        return {
          at: begins,
        };
      const units = FULL_HOUR * share * (0.7 + 0.6 * wobble(key, 1));
      const opus = 0.12 + 0.25 * wobble(key, 2);
      const haiku = 0.002;
      const after = Math.min(1, Math.max(0, (start + HOUR - stepAt) / HOUR));
      return {
        at: begins,
        burn: [
          {
            name: "sonnet",
            units: units * (1 - opus - haiku),
          },
          {
            name: "opus",
            units: units * opus,
          },
          {
            name: "haiku",
            units: units * haiku,
          },
        ],
        model: 41.5 * (0.94 + 0.12 * wobble(key, 3)),
        tools: 5.25 * (0.94 + 0.12 * wobble(key, 4)),
        checks: (512 + (731 - 512) * after) * (0.95 + 0.1 * wobble(key, 5)),
        ...(wobble(key, 6) < 0.35
          ? {}
          : {
              pull_request: 1395 * (0.9 + 0.2 * wobble(key, 7)),
            }),
      };
    },
  );
}
