// The theme tweaks Bridge lays over the theme, over a fake Fleet: only a tweak mod that is on and valid, in name order, with the
// stylesheet `validate_mod` returned for it and no other text, asked for once while the mod stays as it was written.

import { describe, expect, it } from "vitest";
import type { ModChecked, ModSummary } from "@armada/protocol";

import { createFleetTweaks } from "./fleet-tweaks";
import type { FleetTweaks } from "./fleet-tweaks";

const row = (name: string, over: Partial<ModSummary> = {}): ModSummary => ({ name, kind: "tweak", enabled: true, valid: true, changed_at: "t1", ...over });
const settled = () => new Promise((done) => setTimeout(done, 0));
const css = (name: string, at = "t1") => `:root { --x: ${name}-${at}; }`;

function fake(mods: ModSummary[] | null, answer: (name: string, at: number) => Promise<ModChecked | null> = async (name) => ({ name, valid: true, problems: [], css: css(name) })) {
  const asked: string[] = [];
  let hear: Parameters<FleetTweaks["subscribe"]>[0] = () => undefined;
  const fleet: FleetTweaks = {
    state: async () => ({ mods: mods === null ? null : { mods } }),
    subscribe: (on) => ((hear = on), () => undefined),
    validateMod: (name) => (asked.push(name), answer(name, asked.length)),
  };
  const source = createFleetTweaks(fleet);
  return { source, asked, push: (next: ModSummary[]) => hear({ mods: { mods: next } }) };
}

describe("the tweaks laid over the theme", () => {
  it("are the stylesheets Fleet checked for the tweak mods that are on, in mod name order", async () => {
    const { source } = fake([row("beta"), row("zebra"), row("alpha"), row("mid", { enabled: false }), row("bad", { valid: false }), row("theme", { kind: "theme" }), row("jira", { kind: "words" })]);
    await settled();
    await settled();
    expect(source.get()).toEqual([
      { name: "alpha", css: css("alpha") },
      { name: "beta", css: css("beta") },
      { name: "zebra", css: css("zebra") },
    ]);
  });

  it("lay nothing where Fleet's check did not pass, answered nothing, or could not be asked", async () => {
    for (const answer of [{ name: "a", valid: false, problems: ["x"] }, { name: "a", valid: true, problems: [] }, null] as (ModChecked | null)[]) {
      const { source } = fake([row("a")], async () => answer);
      await settled();
      await settled();
      expect(source.get()).toEqual([]);
    }
  });

  it("ask Fleet once for a mod as long as it is as it was written, and again once it is rewritten", async () => {
    const { source, asked, push } = fake([row("a")]);
    await settled();
    push([row("a")]);
    push([row("a", { description: "same file" })]);
    await settled();
    expect(asked).toEqual(["a"]);
    push([row("a", { changed_at: "t2" })]);
    await settled();
    expect(asked).toEqual(["a", "a"]);
    expect(source.get()).toHaveLength(1);
  });

  it("keep the sheet last read until the rewritten one lands, so the screen does not flicker to the theme", async () => {
    let release: (checked: ModChecked) => void = () => undefined;
    let first = true;
    const { source, push } = fake([row("a")], (name) => {
      if (first) {
        first = false;
        return Promise.resolve({ name, valid: true, problems: [], css: css(name) });
      }
      return new Promise((done) => (release = done));
    });
    await settled();
    push([row("a", { changed_at: "t2" })]);
    await settled();
    expect(source.get()).toEqual([{ name: "a", css: css("a") }]);
    release({ name: "a", valid: true, problems: [], css: css("a", "t2") });
    await settled();
    expect(source.get()).toEqual([{ name: "a", css: css("a", "t2") }]);
  });

  it("go when the mod is switched off, and are told to a subscriber once for each change", async () => {
    const { source, push } = fake([row("a")]);
    await settled();
    await settled();
    let heard = 0;
    source.subscribe(() => (heard += 1));
    push([row("a")]);
    expect(heard).toBe(0);
    push([row("a", { enabled: false })]);
    expect(source.get()).toEqual([]);
    expect(heard).toBe(1);
  });
});
