// The Fleet-backed layout source, against a fake Fleet: layout mods folded from the list and read
// from `validate_mod`'s text alone, the owner's choices kept in the `layout_choices` preference and
// outranking every mod, Reset to defaults, and a layout that is invalid or off drawing the defaults.

import { describe, expect, it } from "vitest";

import type { ModChecked, ModSummary, Outcome, Preferences, SavePreference } from "@armada/protocol";
import { layersOf, resolveLayout } from "@armada/shell";
import type { LayoutRegion } from "@armada/shell";

import { createFleetLayout, type FleetLayout } from "./fleet-layout";

const row = (name: string, over: Partial<ModSummary> = {}): ModSummary => ({ name, kind: "layout", enabled: true, valid: true, changed_at: "t1", ...over });
const TIDY = '{"version":1,"dashboard.panels":{"order":["merge-line","fleet"]},"rail":{"hidden":["lessons"]}}';
const REFUSED: Outcome = { ok: false, why: "not_connected" };

type Facts = Parameters<Parameters<FleetLayout["subscribe"]>[0]>[0];
const facts = (mods: ModSummary[] | null, layout_choices?: string): Facts => ({
  mods: mods === null ? null : { mods },
  preferences: { where_things_are_open: false, ...(layout_choices === undefined ? {} : { layout_choices }) } satisfies Preferences,
});

const settled = () => new Promise((done) => setTimeout(done, 0));

function fake(start: Facts, texts: Record<string, string> = { tidy: TIDY }, over: Partial<FleetLayout> = {}) {
  const saved: SavePreference[] = [];
  const checked: string[] = [];
  const asked: string[] = [];
  const told: string[] = [];
  let hear: (facts: Facts) => void = () => undefined;
  const fleet: FleetLayout = {
    state: async () => start,
    subscribe: (on) => ((hear = on), () => undefined),
    validateMod: async (name): Promise<ModChecked> => {
      checked.push(name);
      const layout = texts[name];
      return layout === undefined ? { name, valid: false, problems: ["layout.json is missing"] } : { name, valid: true, problems: [], layout };
    },
    setModEnabled: async (name, enabled) => (asked.push(`${enabled ? "on" : "off"} ${name}`), { ok: true }),
    promoteMod: async (name) => ({ ok: true, modPromoted: { name, branch: `armada/mod-${name}-1`, commit: "abc" } }),
    savePreference: async (save) => (saved.push(save), { ok: true }),
    ...over,
  };
  const source = createFleetLayout(fleet, (sentence) => told.push(sentence));
  const drawn = (region: LayoutRegion) => resolveLayout(region, layersOf(source.get())).shown.map((one) => one.id);
  return { source, push: (next: Facts) => hear(next), saved, checked, asked, told, drawn };
}

describe("layout mods from Fleet's list", () => {
  it("draws the defaults before the list is read, and the mod once Fleet's text arrives", async () => {
    const { source, drawn } = fake(facts([row("tidy")]));
    expect(source.get().mods).toEqual([]);
    await settled();
    expect(source.get().mods.map((mod) => [mod.name, mod.enabled, mod.problem])).toEqual([["tidy", true, undefined]]);
    await settled();
    expect(drawn("dashboard.panels")).toEqual(["merge-line", "fleet"]);
    expect(drawn("rail")).not.toContain("lessons");
  });

  it("lists only layout mods, asks Fleet once per change of a mod, and reads nothing for a mod that is off", async () => {
    const { source, push, checked } = fake(facts([row("tidy"), row("calm", { kind: "theme" }), row("off", { enabled: false })]));
    await settled();
    expect(source.get().mods.map((mod) => mod.name)).toEqual(["tidy", "off"]);
    expect(checked).toEqual(["tidy"]);
    push(facts([row("tidy"), row("off", { enabled: false })], ""));
    await settled();
    expect(checked).toEqual(["tidy"]);
    push(facts([row("tidy", { changed_at: "t2" }), row("off", { enabled: false })]));
    await settled();
    expect(checked).toEqual(["tidy", "tidy"]);
  });

  it("carries Fleet's reason for an invalid mod and draws the defaults for it", async () => {
    const { source, drawn } = fake(facts([row("tidy", { valid: false, reason: "rail.hidden is not a list of up to 32 ids" })]));
    await settled();
    expect(source.get().mods[0]?.problem).toBe("rail.hidden is not a list of up to 32 ids");
    expect(drawn("rail")).toContain("lessons");
  });

  it("refuses text Fleet passed that Bridge's own parser does not, and draws the defaults for it", async () => {
    const { source, drawn } = fake(facts([row("tidy")]), { tidy: '{"version":2}' });
    await settled();
    await settled();
    expect(source.get().mods[0]?.problem).toBe("version is not 1");
    expect(drawn("rail")).toContain("lessons");
  });

  it("applies the text of a mod only from validate_mod, and a mod switched off draws the defaults", async () => {
    const { push, drawn } = fake(facts([row("tidy")]));
    await settled();
    await settled();
    expect(drawn("rail")).not.toContain("lessons");
    push(facts([row("tidy", { enabled: false })]));
    expect(drawn("rail")).toContain("lessons");
  });

  it("keeps the layout it last read while a rewritten mod is read again", async () => {
    const { push, drawn } = fake(facts([row("tidy")]));
    await settled();
    await settled();
    push(facts([row("tidy", { changed_at: "t2" })]));
    expect(drawn("rail")).not.toContain("lessons");
  });

  it("ignores a mod hiding what cannot be hidden, and an id that no longer exists", async () => {
    const greedy = '{"version":1,"rail":{"hidden":["settings","mods","overview","lessons","retired-row"]},"job.tabs":{"first":"retired-tab"}}';
    const { drawn } = fake(facts([row("tidy")]), { tidy: greedy });
    await settled();
    await settled();
    expect(drawn("rail")).toEqual(expect.arrayContaining(["settings", "mods", "overview"]));
    expect(drawn("rail")).not.toContain("lessons");
    expect(drawn("job.tabs")[0]).toBe("overview");
  });
});

describe("the owner's own choices", () => {
  it("read from the preference, parsed by the same rules, and outranking a mod", async () => {
    const own = '{"version":1,"rail":{"hidden":[]}}';
    const { source, drawn } = fake(facts([row("tidy")], own));
    await settled();
    await settled();
    expect(source.get().own.regions).toEqual({ rail: { hidden: [] } });
    expect(drawn("rail")).toContain("lessons");
    expect(drawn("dashboard.panels")).toEqual(["merge-line", "fleet"]);
  });

  it("text that does not parse is no choices", async () => {
    const { source, drawn } = fake(facts([], '{"version":9,"rail":{"hidden":["checks"]}}'));
    await settled();
    expect(source.get().own.regions).toEqual({});
    expect(drawn("rail")).toContain("checks");
  });

  it("saves a choice as layout_choices text at once, and keeps the other regions", async () => {
    const { source, saved, drawn } = fake(facts([], '{"version":1,"job.tabs":{"first":"plan"}}'));
    await settled();
    source.setOwn("rail", { hidden: ["checks"] });
    expect(drawn("rail")).not.toContain("checks");
    await settled();
    expect(saved).toHaveLength(1);
    expect(saved[0]?.name).toBe("layout_choices");
    expect(JSON.parse(saved[0]?.text ?? "")).toEqual({ version: 1, "job.tabs": { first: "plan" }, rail: { hidden: ["checks"] } });
  });

  it("takes a region's choice back, and saves empty text when none is left", async () => {
    const { source, saved } = fake(facts([], '{"version":1,"rail":{"hidden":["checks"]}}'));
    await settled();
    source.setOwn("rail", undefined);
    await settled();
    expect(saved[0]?.text).toBe("");
  });

  it("puts the old choices back and says why when Fleet refuses", async () => {
    const { source, told, drawn } = fake(facts([], '{"version":1,"rail":{"hidden":["checks"]}}'), {}, { savePreference: async () => REFUSED });
    await settled();
    source.setOwn("rail", { hidden: ["checks", "kit"] });
    await settled();
    expect(drawn("rail")).toContain("kit");
    expect(drawn("rail")).not.toContain("checks");
    expect(told).toHaveLength(1);
  });
});

describe("Reset to defaults", () => {
  it("clears the owner's choices and switches every layout mod that is on off, leaving them installed", async () => {
    const { source, saved, asked, drawn } = fake(facts([row("tidy"), row("quiet", { enabled: false }), row("calm", { kind: "theme" })], '{"version":1,"rail":{"hidden":["checks"]}}'));
    await settled();
    await settled();
    source.toDefaults();
    expect(drawn("rail")).toContain("checks");
    expect(drawn("rail")).toContain("lessons");
    await settled();
    expect(saved).toEqual([{ name: "layout_choices", value: false, text: "" }]);
    expect(asked).toEqual(["off tidy"]);
    expect(source.get().mods.map((mod) => mod.name)).toEqual(["tidy", "quiet"]);
  });
});

describe("the acts on a mod", () => {
  it("switch it, and promote it to the branch Fleet names", async () => {
    const { source, asked } = fake(facts([row("tidy")]));
    await settled();
    source.setEnabled("tidy", false);
    await settled();
    expect(asked).toEqual(["off tidy"]);
    source.promote("tidy");
    await settled();
    expect(source.get().mods[0]?.branch).toBe("armada/mod-tidy-1");
  });
});
