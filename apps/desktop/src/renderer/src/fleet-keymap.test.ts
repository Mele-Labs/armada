// The owner's key bindings over Fleet: what the `key_bindings` preference holds is laid over the
// registry, a change is saved back as that preference, and a refused save puts Fleet's keys back.

import { afterEach, describe, expect, it } from "vitest";
import type { Outcome, Preferences, SavePreference } from "@armada/protocol";
import { keyFor, parseSlots, rebind, setKeyBindingSaver, setKeyBindings } from "@armada/components";

import { wireFleetKeymap, type FleetKeymap } from "./fleet-keymap";

const prefs = (key_bindings?: string): { preferences: Preferences } => ({
  preferences: { where_things_are_open: false, ...(key_bindings === undefined ? {} : { key_bindings }) },
});

function fleet(start: string | undefined, answer: Outcome) {
  let hear: (facts: { preferences: Preferences }) => void = () => undefined;
  const saved: SavePreference[] = [];
  const told: string[] = [];
  const api: FleetKeymap = {
    state: async () => prefs(start),
    subscribe: (on) => ((hear = on), () => undefined),
    savePreference: async (save) => (saved.push(save), answer),
  };
  wireFleetKeymap(api, (sentence) => told.push(sentence));
  return { saved, told, publish: (text?: string) => hear(prefs(text)) };
}

const settle = () => new Promise((done) => setTimeout(done, 0));

afterEach(() => {
  setKeyBindingSaver(null);
  setKeyBindings({});
});

describe("key bindings over Fleet", () => {
  it("lays what Fleet holds over the registry, and follows what it publishes", async () => {
    const one = fleet('{"version":1,"bindings":{"kill":["q"]}}', { ok: true });
    await settle();
    expect(keyFor("kill")).toBe("q");
    one.publish("");
    expect(keyFor("kill")).toBe("x");
  });

  it("saves a change as the key_bindings preference", async () => {
    const one = fleet(undefined, { ok: true });
    await settle();
    rebind("kill", parseSlots("q"));
    expect(one.saved).toEqual([{ name: "key_bindings", value: false, text: '{"version":1,"bindings":{"kill":["q"]}}' }]);
  });

  it("puts Fleet's keys back and says why when the save is refused", async () => {
    const refused: Outcome = { ok: false, why: "not_connected" };
    const one = fleet(undefined, refused);
    await settle();
    rebind("kill", parseSlots("q"));
    expect(keyFor("kill")).toBe("q");
    await settle();
    expect(keyFor("kill")).toBe("x");
    expect(one.told).toHaveLength(1);
  });
});
