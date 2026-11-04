// The last 48 hours, one point an hour, as the Usage page's two line charts draw them. These types are the
// page's own: the window turns Fleet's hours into them (`usage-time-wire.ts`).

import type { NamedUnits } from "@armada/protocol";

import type { Build } from "./usage-builds";

export const HOUR_MS = 3_600_000;
export const SHOWN = 48;

/** One hour. A figure an hour did not produce is absent, so the line breaks there rather than falling to zero. */
export type UsageHour = {
  /** The hour's start, as an instant. */
  at: string;
  /** Cost units, millions, by model. Absent where nothing ran. */
  burn?: readonly NamedUnits[];
  /** Seconds: the median wait on the model per turn, a tool call, a Check run and a pull request's wait in the hour. */
  model?: number;
  tools?: number;
  checks?: number;
  pull_request?: number;
};

/** Oldest first, `SHOWN` hours. */
export type UsageHours = readonly UsageHour[];

/** The hour an instant falls in, as an index into the 48; below 0 or from `SHOWN` it is outside them. */
export const hourIndex = (hours: UsageHours, instant: string): number =>
  Math.floor((Date.parse(instant) - Date.parse(hours[0]!.at)) / HOUR_MS);

/** An instant as a position along the axis, in hours from the first hour's start. */
export const hourOffset = (hours: UsageHours, instant: string): number =>
  (Date.parse(instant) - Date.parse(hours[0]!.at)) / HOUR_MS;

/** A build's marker: where on the axis it begins and where its span ends, which is the next build's start or the axis's end. */
export type Marker = {
  build: Build;
  /** Hours from the axis's start. */
  at: number;
  /** Where the span to the next marker ends. */
  to: number;
};

/** The builds that began inside the 48 hours, oldest first, each with the span it ran for. */
export function markersOf(hours: UsageHours, builds: readonly Build[]): Marker[] {
  const inside = builds
    .map((build) => ({
      build,
      at: hourOffset(hours, build.started),
    }))
    .filter((one) => one.at >= 0 && one.at <= SHOWN)
    .sort((a, b) => a.at - b.at);
  return inside.map((one, at) => ({
    ...one,
    to: inside[at + 1]?.at ?? SHOWN,
  }));
}

const CLOCK = (at: Date): string =>
  `${String(at.getHours()).padStart(2, "0")}:${String(at.getMinutes()).padStart(2, "0")}`;

/** An hour as `Nov 3, 14:00 to 15:00`. */
export function hourName(hours: UsageHours, index: number): string {
  const from = new Date(Date.parse(hours[0]!.at) + index * HOUR_MS);
  const to = new Date(from.getTime() + HOUR_MS);
  const months = [
    "Jan",
    "Feb",
    "Mar",
    "Apr",
    "May",
    "Jun",
    "Jul",
    "Aug",
    "Sep",
    "Oct",
    "Nov",
    "Dec",
  ];
  return `${months[from.getMonth()]} ${from.getDate()}, ${CLOCK(from)} to ${CLOCK(to)}`;
}

/** The local weekday of an hour, Monday first, as the day and hour map numbers them. */
export const weekdayOfHour = (hours: UsageHours, index: number): number =>
  (new Date(Date.parse(hours[0]!.at) + index * HOUR_MS).getDay() + 6) % 7;

/** The map's three hour block an hour falls in. */
export const blockOfHour = (hours: UsageHours, index: number): number =>
  Math.floor(new Date(Date.parse(hours[0]!.at) + index * HOUR_MS).getHours() / 3);

/** The latest hour that holds anything, or the last hour where none does. */
export const latestHour = (hours: UsageHours): number => {
  for (let at = hours.length - 1; at >= 0; at--) {
    const one = hours[at]!;
    if (one.burn !== undefined) return at;
  }
  return hours.length - 1;
};
