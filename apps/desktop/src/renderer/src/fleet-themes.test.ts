// The Fleet-backed theme source, against a fake Fleet: the list folded as `mods.changed` carries
// it, a mod's CSS taken from `validate_mod` and nowhere else, and the choice kept in Fleet's
// preference. What `Themed` does with the source is `theme-loader.test.tsx`'s.

import { describe, expect, it } from "vitest";

import type { ModChecked, ModSummary, Outcome, Preferences, SavePreference } from "@armada/protocol";
import type { CatalogueTheme } from "@armada/settings";

import { createFleetThemes, type FleetThemes } from "./fleet-themes";

const row = (name: string, over: Partial<ModSummary> = {}): ModSummary => ({ name, kind: "theme", enabled: true, valid: true, ...over });
const NORD: CatalogueTheme = { id: "catalogue:nord", title: "Nord", tone: "dark", load: () => Promise.resolve("") };
const REFUSED: Outcome = { ok: false, why: "not_connected" };

type Facts = Parameters<Parameters<FleetThemes["subscribe"]>[0]>[0];
const facts = (mods: ModSummary[] | null, theme?: string): Facts => ({
  mods: mods === null ? null : { mods },
  preferences: { where_things_are_open: false, ...(theme === undefined ? {} : { theme }) } satisfies Preferences,
});

const settled = () => new Promise((done) => setTimeout(done, 0));

/** A Fleet with a list, a preference, and a record of what Bridge asked of it. */
function fake(start: Facts, over: Partial<FleetThemes> = {}) {
  const saved: SavePreference[] = [];
  const checked: string[] = [];
  const asked: string[] = [];
  const told: string[] = [];
  let hear: (facts: Facts) => void = () => undefined;
  const fleet: FleetThemes = {
    state: async () => start,
    subscribe: (on) => ((hear = on), () => undefined),
    validateMod: async (name) => (checked.push(name), { name, valid: true, problems: [], css: `:root { --bg-base: #123456; } /* ${name} */` }),
    setModEnabled: async (name, enabled) => (asked.push(`${enabled ? "on" : "off"} ${name}`), { ok: true }),
    promoteMod: async (name) => ({ ok: true, modPromoted: { name, branch: `armada/mod-${name}-1`, commit: "abc" } }),
    savePreference: async (save) => (saved.push(save), { ok: true }),
    ...over,
  };
  const source = createFleetThemes(fleet, [NORD], (sentence) => told.push(sentence));
  return { source, push: (next: Facts) => hear(next), saved, checked, asked, told };
}

describe("the list, folded as mods.changed carries it", () => {
  it("offers nothing before the list is read, then each mod by name, and replaces the list whole", async () => {
    const { source, push } = fake(facts(null));
    await settled();
    expect(source.get().mods).toEqual([]);
    push(facts([row("calm"), row("warm", { enabled: false })]));
    expect(source.get().mods.map((mod) => [mod.name, mod.enabled])).toEqual([["calm", true], ["warm", false]]);
    push(facts([row("warm")]));
    expect(source.get().mods.map((mod) => [mod.name, mod.enabled])).toEqual([["warm", true]]);
  });

  it("tells its subscribers once per change to the mods or the theme, and not for the rest of the state", async () => {
    const { source, push } = fake(facts([row("calm")]));
    await settled();
    let heard = 0;
    source.subscribe(() => (heard += 1));
    push(facts([row("calm")]));
    push(facts([row("calm")]));
    expect(heard).toBe(0);
    push(facts([row("calm"), row("warm")]));
    expect(heard).toBe(1);
    push(facts([row("calm"), row("warm")], "light"));
    expect(heard).toBe(2);
  });

  it("carries Fleet's reason on a mod that is not valid, and a mod that is on but invalid is not offered", async () => {
    const { source } = fake(facts([row("bad", { valid: false, reason: "line 3: not a token" }), row("calm")]));
    await settled();
    const bad = source.get().mods.find((mod) => mod.name === "bad");
    expect(bad?.problem).toBe("line 3: not a token");
    expect(bad?.enabled).toBe(true);
    expect(source.get().mods.find((mod) => mod.name === "calm")?.problem).toBeUndefined();
  });
});

describe("a mod's CSS", () => {
  it("is exactly what validate_mod returned, asked for when the theme is drawn", async () => {
    const { source, checked } = fake(facts([row("calm")]));
    await settled();
    expect(checked).toEqual([]);
    const css = await source.get().mods[0]!.load();
    expect(css).toBe(":root { --bg-base: #123456; } /* calm */");
    expect(checked).toEqual(["calm"]);
  });

  it("is refused where Fleet's check did not pass, answered nothing, or could not be asked", async () => {
    const answers: (ModChecked | null)[] = [
      { name: "calm", valid: false, problems: ["line 2: `url(` is not allowed"] },
      { name: "calm", valid: true, problems: [] },
      null,
    ];
    for (const answer of answers) {
      const { source } = fake(facts([row("calm")]), { validateMod: async () => answer });
      await settled();
      await expect(source.get().mods[0]!.load()).rejects.toThrow();
    }
  });
});

describe("the choice, kept in Fleet's theme preference", () => {
  it("is read from the preference, and a mod the preference names is shown once the list says it can be", async () => {
    const { source, push, saved } = fake(facts(null, "calm"));
    await settled();
    // Not read yet: Dark is shown, and the choice is not forgotten for want of a list.
    expect(source.get().active).toBe("dark");
    push(facts([row("calm")], "calm"));
    expect(source.get().active).toBe("calm");
    expect(saved).toEqual([]);
  });

  it("falls back to Dark for a mod that is invalid, keeps the choice, and returns when the mod passes", async () => {
    const { source, push, saved } = fake(facts([row("calm", { valid: false, reason: "line 1: x" })], "calm"));
    await settled();
    expect(source.get().active).toBe("dark");
    push(facts([row("calm")], "calm"));
    expect(source.get().active).toBe("calm");
    expect(saved).toEqual([]);
  });

  it("saves a pick as text, a catalogue id as spelled, and a built-in as itself", async () => {
    const { source, saved } = fake(facts([row("calm")]));
    await settled();
    source.setActive("calm");
    source.setActive("catalogue:nord");
    source.setActive("light");
    await settled();
    expect(saved).toEqual([
      { name: "theme", value: false, text: "calm" },
      { name: "theme", value: false, text: "catalogue:nord" },
      { name: "theme", value: false, text: "light" },
    ]);
    expect(source.get().active).toBe("light");
  });

  it("chooses Dark for an id nothing offers, and for a mod that cannot be drawn", async () => {
    const { source, saved } = fake(facts([row("bad", { valid: false, reason: "x" })]));
    await settled();
    source.setActive("gone");
    source.setActive("bad");
    await settled();
    expect(saved.map((one) => one.text)).toEqual(["dark", "dark"]);
  });

  it("puts the old choice back and says why when Fleet refuses the save", async () => {
    const { source, told } = fake(facts([row("calm")], "light"), { savePreference: async () => REFUSED });
    await settled();
    source.setActive("calm");
    await settled();
    expect(source.get().active).toBe("light");
    expect(told).toHaveLength(1);
  });

  it("is forgotten when the chosen mod is switched off or gone, and not when it only fails a check", async () => {
    const off = fake(facts([row("calm")], "calm"));
    await settled();
    off.push(facts([row("calm", { enabled: false })], "calm"));
    await settled();
    expect(off.saved).toEqual([{ name: "theme", value: false, text: "dark" }]);

    const gone = fake(facts([row("calm")], "calm"));
    await settled();
    gone.push(facts([], "calm"));
    await settled();
    expect(gone.saved).toEqual([{ name: "theme", value: false, text: "dark" }]);
  });

  it("never makes a mod the theme because it appeared", async () => {
    const { source, push, saved } = fake(facts([], "dark"));
    await settled();
    push(facts([row("fresh")], "dark"));
    expect(source.get().mods.map((mod) => mod.name)).toEqual(["fresh"]);
    expect(source.get().active).toBe("dark");
    expect(saved).toEqual([]);
  });

  it("keeps the choice when the loader could not draw it, and tries again when the list changes", async () => {
    const { source, push, saved } = fake(facts([row("calm")], "calm"));
    await settled();
    expect(source.get().active).toBe("calm");
    source.fellBack?.("calm");
    expect(source.get().active).toBe("dark");
    expect(saved).toEqual([]);
    push(facts([row("calm", { changed_at: "2026-10-08T22:00:00.000Z" })], "calm"));
    expect(source.get().active).toBe("calm");
  });
});

describe("the acts on a mod", () => {
  it("switches one through Fleet, and says what Fleet refused", async () => {
    const { source, asked, told } = fake(facts([row("calm")]));
    await settled();
    source.setEnabled("calm", false);
    await settled();
    expect(asked).toEqual(["off calm"]);
    expect(told).toEqual([]);

    const refusing = fake(facts([row("calm")]), { setModEnabled: async () => REFUSED });
    await settled();
    refusing.source.setEnabled("calm", false);
    await settled();
    expect(refusing.told).toHaveLength(1);
  });

  it("promotes one and holds the branch Fleet named, as the plain fact", async () => {
    const { source } = fake(facts([row("calm")]));
    await settled();
    expect(source.get().mods[0]?.branch).toBeUndefined();
    source.promote("calm");
    await settled();
    expect(source.get().mods[0]?.branch).toBe("armada/mod-calm-1");
  });
});
