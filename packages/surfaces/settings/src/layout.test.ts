// The layout rules Bridge applies: `parseLayout` agrees with Fleet's rules on what is invalid, the
// shipped registry file agrees with the registry Bridge draws from, and a layer outranks the one
// under it. `crates/ipc/src/layout.rs` is Fleet's half, and `packages/shell/layout-registry.json`
// is the list both check.

import { describe, expect, it } from "vitest";

import registry from "@armada/shell/layout-registry.json";
import { LAYOUT, LAYOUT_BYTES, LAYOUT_REGIONS, layersOf, parseLayout, resolveLayout, serializeLayout } from "@armada/shell";
import type { LayoutFile, LayoutMod } from "@armada/shell";

const mod = (name: string, text: string, over: Partial<LayoutMod> = {}): LayoutMod => ({ name, title: name, enabled: true, file: parseLayout(text).file, ...over });
const shown = (region: Parameters<typeof resolveLayout>[0], layers: Parameters<typeof resolveLayout>[1]) => resolveLayout(region, layers).shown.map((one) => one.id);

describe("the registry file both sides check", () => {
  it("names the regions, ids, order and hideability that Bridge's registry has", () => {
    expect(Object.keys(registry.regions)).toEqual(LAYOUT_REGIONS);
    for (const region of LAYOUT_REGIONS) {
      const shipped = registry.regions[region];
      const spec = LAYOUT[region];
      expect({ region, ordered: shipped.ordered, firstable: shipped.firstable }).toEqual({ region, ordered: spec.ordered, firstable: spec.firstable });
      expect(shipped.entries).toEqual(spec.entries.map((one) => ({ id: one.id, hideable: one.hideable })));
    }
  });
});

describe("the examples the armada-mods skill teaches", () => {
  // Written out as the skill has them: a skill is not a file a test may read, so a change to either is a change to both.
  const tidy = '{\n  "version": 1,\n  "dashboard.panels": { "hidden": ["merge-line", "activity"] },\n  "rail": { "hidden": ["logbook"] }\n}\n';
  const job = '{\n  "version": 1,\n  "dashboard.panels": { "hidden": ["field"] },\n  "job.panels": { "hidden": ["next"] }\n}\n';
  it("pass, and do what the skill says they do", () => {
    expect(parseLayout(tidy).problems).toEqual([]);
    expect(parseLayout(job).problems).toEqual([]);
    // Each is its own mod: two mods naming the same region would have the later name's list replace the earlier one's.
    const tidied = layersOf({ mods: [mod("tidy", tidy)], own: { regions: {} } });
    expect(shown("dashboard.panels", tidied)).toEqual(["list", "field", "bays", "controls", "dispatch", "autopilot"]);
    expect(shown("rail", tidied)).not.toContain("logbook");
    const jobs = layersOf({ mods: [mod("jobs", job)], own: { regions: {} } });
    expect(shown("dashboard.panels", jobs)).toEqual(["list", "merge-line", "activity", "bays", "controls", "dispatch", "autopilot"]);
    expect(shown("job.panels", jobs)).toEqual(["done", "now"]);
  });
});

describe("parseLayout refuses what Fleet refuses", () => {
  const long = (n: number) => JSON.stringify({ version: 1, rail: { hidden: Array(n).fill("logbook") } });
  it.each([
    ["", "not JSON"],
    ["[]", "not an object"],
    ["{}", "version is not 1"],
    ['{"version":2}', "version is not 1"],
    ['{"version":"1"}', "version is not 1"],
    ['{"version":1,"rail":[]}', "rail is not an object"],
    ['{"version":1,"rail":{"colour":"red"}}', "rail.colour is not a field"],
    ['{"version":1,"rail":{"hidden":"logbook"}}', "rail.hidden is not a list"],
    ['{"version":1,"rail":{"hidden":["Lessons"]}}', "rail.hidden is not a list"],
    ['{"version":1,"job.panels":{"first":["now"]}}', "job.panels.first is not an id"],
    [long(33), "rail.hidden is not a list"],
    [`{"version":1,"x":"${"a".repeat(LAYOUT_BYTES)}"}`, `larger than ${LAYOUT_BYTES} bytes`],
  ])("%s", (text, why) => {
    const parsed = parseLayout(text);
    expect(parsed.problems.join("; ")).toContain(why);
    expect(parsed.file.regions).toEqual({});
  });

  it("accepts a file at the limit of 32 ids, ignores what this build does not have, and names it", () => {
    expect(parseLayout(long(32)).problems).toEqual([]);
    const parsed = parseLayout('{"version":1,"sidebar":7,"job.tabs":{"first":"plan"},"rail":{"hidden":["gone","logbook"],"order":["logbook"]},"job.panels":{"first":"now"}}');
    expect(parsed.problems).toEqual([]);
    expect(parsed.file.regions).toEqual({ rail: { hidden: ["logbook"] }, "job.panels": {} });
    expect(parsed.ignored).toEqual(expect.arrayContaining(["sidebar", "job.tabs", "rail.hidden gone", "rail.order", "job.panels.first"]));
  });
});

describe("serializeLayout", () => {
  it("is empty for nothing chosen, and reads back as what was chosen", () => {
    expect(serializeLayout({ regions: {} })).toBe("");
    expect(serializeLayout({ regions: { rail: {} } })).toBe("");
    const file: LayoutFile = { regions: { "job.panels": { hidden: ["next"] }, rail: { hidden: ["logbook"] } } };
    const text = serializeLayout(file);
    expect(parseLayout(text).problems).toEqual([]);
    expect(parseLayout(text).file).toEqual(file);
  });
});

describe("what is drawn, layer on layer", () => {
  const TIDY = '{"version":1,"job.panels":{"hidden":["next"]},"rail":{"hidden":["logbook"]}}';

  it("is the registry's own order and every entry with no layer", () => {
    expect(shown("dashboard.panels", layersOf({ mods: [], own: { regions: {} } }))).toEqual(["list", "field", "merge-line", "activity", "bays", "controls", "dispatch", "autopilot"]);
    expect(shown("job.panels", layersOf({ mods: [], own: { regions: {} } }))).toEqual(["done", "now", "next"]);
  });

  it("applies a mod that is on and valid, and not one that is off or has a problem", () => {
    const on = layersOf({ mods: [mod("tidy", TIDY)], own: { regions: {} } });
    expect(shown("job.panels", on)).toEqual(["done", "now"]);
    expect(shown("rail", on)).not.toContain("logbook");
    const off = layersOf({ mods: [mod("tidy", TIDY, { enabled: false })], own: { regions: {} } });
    expect(shown("rail", off)).toContain("logbook");
    const broken = layersOf({ mods: [mod("tidy", TIDY, { problem: "line 3" })], own: { regions: {} } });
    expect(shown("rail", broken)).toContain("logbook");
  });

  it("applies mods in name order, so a later name's list replaces an earlier one's", () => {
    const a = mod("a-first", '{"version":1,"job.panels":{"hidden":["done"]}}');
    const z = mod("z-last", '{"version":1,"job.panels":{"hidden":["next"]}}');
    expect(shown("job.panels", layersOf({ mods: [z, a], own: { regions: {} } }))).toEqual(["done", "now"]);
  });

  it("puts the owner's own choice over every mod, in both directions", () => {
    const hides = mod("tidy", TIDY);
    const showsIt: LayoutFile = { regions: { rail: { hidden: [] } } };
    expect(shown("rail", layersOf({ mods: [hides], own: showsIt }))).toContain("logbook");
    const hidesIt: LayoutFile = { regions: { rail: { hidden: ["checks"] } } };
    const result = shown("rail", layersOf({ mods: [mod("tidy", '{"version":1}')], own: hidesIt }));
    expect(result).not.toContain("checks");
    const shownAll: LayoutFile = { regions: { "job.panels": { hidden: [] } } };
    expect(shown("job.panels", layersOf({ mods: [hides], own: shownAll }))).toEqual(["done", "now", "next"]);
  });

  it("never hides what cannot be hidden, whoever asks", () => {
    const asks = mod("greedy", '{"version":1,"rail":{"hidden":["settings","mods","overview","logbook"]},"job.panels":{"hidden":["now","next"]},"dashboard.panels":{"hidden":["list","bays"]}}');
    const layers = layersOf({ mods: [asks], own: { regions: {} } });
    expect(shown("rail", layers)).toEqual(expect.arrayContaining(["settings", "mods", "overview"]));
    expect(shown("rail", layers)).not.toContain("logbook");
    expect(shown("job.panels", layers)).toEqual(["done", "now"]);
    expect(shown("dashboard.panels", layers)).toContain("list");
    expect(shown("dashboard.panels", layers)).not.toContain("bays");
  });

  it("skips an id that is gone, and never orders the Cockpit's panels or a Job's", () => {
    const stale = mod("old", '{"version":1,"job.panels":{"order":["removed-panel","done"],"hidden":["removed-panel","done"]},"dashboard.panels":{"order":["bays","list"]}}');
    const layers = layersOf({ mods: [stale], own: { regions: {} } });
    const resolved = resolveLayout("job.panels", layers);
    expect(resolved.all.map((one) => one.id)).toEqual(["done", "now", "next"]);
    expect(resolved.shown.map((one) => one.id)).toEqual(["now", "next"]);
    expect(shown("dashboard.panels", layers)[0]).toBe("list");
  });
});
