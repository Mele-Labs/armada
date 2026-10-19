// The owner's time format over Fleet: what the `time_format` preference holds is the format every stamp
// is drawn in, a change is saved back as that preference, and a refused save puts Fleet's format back.

import { afterEach, describe, expect, it } from "vitest";
import type { Outcome, Preferences, SavePreference } from "@armada/protocol";
import { DEFAULT_TIME_FORMAT, saveTimeFormat, setTimeFormat, setTimeFormatSaver, timeFormat } from "@armada/components";

import { wireFleetTimeFormat, type FleetTimeFormat } from "./fleet-time-format";

const prefs = (time_format?: string): { preferences: Preferences } => ({
  preferences: { where_things_are_open: false, ...(time_format === undefined ? {} : { time_format }) },
});

function fleet(start: string | undefined, answer: Outcome) {
  let hear: (facts: { preferences: Preferences }) => void = () => undefined;
  const saved: SavePreference[] = [];
  const told: string[] = [];
  const api: FleetTimeFormat = {
    state: async () => prefs(start),
    subscribe: (on) => ((hear = on), () => undefined),
    savePreference: async (save) => (saved.push(save), answer),
  };
  wireFleetTimeFormat(api, (sentence) => told.push(sentence));
  return { saved, told, publish: (text?: string) => hear(prefs(text)) };
}

const settle = () => new Promise((done) => setTimeout(done, 0));
const TWELVE = { hour12: true, seconds: false, date: true };

afterEach(() => {
  setTimeFormatSaver(null);
  setTimeFormat(DEFAULT_TIME_FORMAT);
});

describe("time format over Fleet", () => {
  it("draws in what Fleet holds, and follows what it publishes", async () => {
    const one = fleet(JSON.stringify(TWELVE), { ok: true });
    await settle();
    expect(timeFormat()).toEqual(TWELVE);
    one.publish("");
    expect(timeFormat()).toEqual(DEFAULT_TIME_FORMAT);
  });

  it("saves a change as the time_format preference", async () => {
    const one = fleet(undefined, { ok: true });
    await settle();
    saveTimeFormat(TWELVE);
    expect(one.saved).toEqual([{ name: "time_format", value: false, text: JSON.stringify(TWELVE) }]);
  });

  it("puts Fleet's format back and says why when the save is refused", async () => {
    const refused: Outcome = { ok: false, why: "not_connected" };
    const one = fleet(undefined, refused);
    await settle();
    saveTimeFormat(TWELVE);
    expect(timeFormat()).toEqual(TWELVE);
    await settle();
    expect(timeFormat()).toEqual(DEFAULT_TIME_FORMAT);
    expect(one.told).toHaveLength(1);
  });
});
