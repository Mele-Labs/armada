// The layout rules Bridge applies: `parseLayout` agrees with Fleet's rules on what is invalid, the
// shipped registry file agrees with the registry Bridge draws from, and a layer outranks the one
// under it. `crates/ipc/src/layout.rs` is Fleet's half, and `packages/shell/layout-registry.json`
// is the list both check.

import { describe, expect, it } from "vitest";

import registry from "@armada/shell/layout-registry.json";
import skill from "../../../../.claude/skills/armada-mods/SKILL.md?raw";
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

describe("the regions table in the armada-mods skill", () => {
  const ids = (cell = "") => [...cell.matchAll(/`([^`]+)`/g)].map((one) => one[1]);
  const rows = skill
    .split("\n")
    .map((line) => line.split("|").map((cell) => cell.trim()))
    .filter((cells) => cells.length === 7 && /^`[a-z.]+`$/.test(cells[1] ?? ""))
    .map((cells) => ({ region: ids(cells[1])[0], ids: ids(cells[2]), fixed: ids(cells[3]), order: cells[4], first: cells[5] }));
  it("names the registry's regions, ids, non-hideable ids and flags", () => {
    expect(rows.map((row) => row.region)).toEqual(Object.keys(registry.regions));
    for (const row of rows) {
      const shipped = registry.regions[row.region as keyof typeof registry.regions];
      expect(row.ids).toEqual(shipped.entries.map((one) => one.id));
      expect(row.fixed).toEqual(shipped.entries.filter((one) => !one.hideable).map((one) => one.id));
      expect({ order: row.order, first: row.first }).toEqual({ order: shipped.ordered ? "yes" : "no", first: shipped.firstable ? "yes" : "no" });
    }
  });
});

describe("the examples the armada-mods skill teaches", () => {
  // Written out as the skill has them, so a change to either is a change to both.
  const tidy = '{\n  "version": 1,\n  "dashboard.panels": { "order": ["merge-line", "fleet"] },\n  "rail": { "hidden": ["lessons"] }\n}\n';
  const job = '{\n  "version": 1,\n  "job.tabs": { "order": ["overview", "workflow", "record", "plan"], "hidden": ["pulse"], "first": "plan" }\n}\n';
  it("pass, and do what the skill says they do", () => {
    expect(parseLayout(tidy).problems).toEqual([]);
    expect(parseLayout(job).problems).toEqual([]);
    const layers = layersOf({ mods: [mod("tidy", tidy), mod("jobs", job)], own: { regions: {} } });
    expect(shown("dashboard.panels", layers)[0]).toBe("merge-line");
    expect(shown("rail", layers)).not.toContain("lessons");
    expect(shown("job.tabs", layers).slice(0, 4)).toEqual(["overview", "workflow", "record", "plan"]);
    expect(shown("job.tabs", layers)).not.toContain("pulse");
    expect(resolveLayout("job.tabs", layers).first).toBe("plan");
  });
});

describe("parseLayout refuses what Fleet refuses", () => {
  const long = (n: number) => JSON.stringify({ version: 1, rail: { hidden: Array(n).fill("lessons") } });
  it.each([
    ["", "not JSON"],
    ["[]", "not an object"],
    ["{}", "version is not 1"],
    ['{"version":2}', "version is not 1"],
    ['{"version":"1"}', "version is not 1"],
    ['{"version":1,"rail":[]}', "rail is not an object"],
    ['{"version":1,"rail":{"colour":"red"}}', "rail.colour is not a field"],
    ['{"version":1,"rail":{"hidden":"lessons"}}', "rail.hidden is not a list"],
    ['{"version":1,"rail":{"hidden":["Lessons"]}}', "rail.hidden is not a list"],
    ['{"version":1,"job.tabs":{"first":["plan"]}}', "job.tabs.first is not an id"],
    [long(33), "rail.hidden is not a list"],
    [`{"version":1,"x":"${"a".repeat(LAYOUT_BYTES)}"}`, `larger than ${LAYOUT_BYTES} bytes`],
  ])("%s", (text, why) => {
    const parsed = parseLayout(text);
    expect(parsed.problems.join("; ")).toContain(why);
    expect(parsed.file.regions).toEqual({});
  });

  it("accepts a file at the limit of 32 ids, ignores what this build does not have, and names it", () => {
    expect(parseLayout(long(32)).problems).toEqual([]);
    const parsed = parseLayout('{"version":1,"sidebar":7,"rail":{"hidden":["gone","lessons"],"order":["lessons"]},"job.tabs":{"first":"gone"}}');
    expect(parsed.problems).toEqual([]);
    expect(parsed.file.regions).toEqual({ rail: { hidden: ["lessons"] }, "job.tabs": {} });
    expect(parsed.ignored).toEqual(expect.arrayContaining(["sidebar", "rail.hidden gone", "rail.order", "job.tabs.first gone"]));
  });
});

describe("serializeLayout", () => {
  it("is empty for nothing chosen, and reads back as what was chosen", () => {
    expect(serializeLayout({ regions: {} })).toBe("");
    expect(serializeLayout({ regions: { rail: {} } })).toBe("");
    const file: LayoutFile = { regions: { "dashboard.panels": { order: ["merge-line", "fleet"] }, rail: { hidden: ["lessons"] } } };
    const text = serializeLayout(file);
    expect(parseLayout(text).problems).toEqual([]);
    expect(parseLayout(text).file).toEqual(file);
  });
});

describe("what is drawn, layer on layer", () => {
  const TIDY = '{"version":1,"dashboard.panels":{"order":["merge-line","fleet"]},"rail":{"hidden":["lessons"]}}';

  it("is the registry's own order and every entry with no layer", () => {
    expect(shown("dashboard.panels", layersOf({ mods: [], own: { regions: {} } }))).toEqual(["quick-dispatch", "fleet", "merge-line"]);
  });

  it("applies a mod that is on and valid, and not one that is off or has a problem", () => {
    const on = layersOf({ mods: [mod("tidy", TIDY)], own: { regions: {} } });
    expect(shown("dashboard.panels", on)).toEqual(["merge-line", "fleet", "quick-dispatch"]);
    expect(shown("rail", on)).not.toContain("lessons");
    const off = layersOf({ mods: [mod("tidy", TIDY, { enabled: false })], own: { regions: {} } });
    expect(shown("rail", off)).toContain("lessons");
    const broken = layersOf({ mods: [mod("tidy", TIDY, { problem: "line 3" })], own: { regions: {} } });
    expect(shown("rail", broken)).toContain("lessons");
  });

  it("applies mods in name order, so a later name's list replaces an earlier one's", () => {
    const a = mod("a-first", '{"version":1,"job.tabs":{"order":["record","overview"]}}');
    const z = mod("z-last", '{"version":1,"job.tabs":{"order":["checks","overview"]}}');
    expect(shown("job.tabs", layersOf({ mods: [z, a], own: { regions: {} } })).slice(0, 2)).toEqual(["checks", "overview"]);
  });

  it("puts the owner's own choice over every mod, in both directions", () => {
    const hides = mod("tidy", TIDY);
    const showsIt: LayoutFile = { regions: { rail: { hidden: [] } } };
    expect(shown("rail", layersOf({ mods: [hides], own: showsIt }))).toContain("lessons");
    const hidesIt: LayoutFile = { regions: { rail: { hidden: ["checks"] } } };
    const result = shown("rail", layersOf({ mods: [mod("tidy", '{"version":1}')], own: hidesIt }));
    expect(result).not.toContain("checks");
    const ordered: LayoutFile = { regions: { "dashboard.panels": { order: ["fleet", "merge-line"] } } };
    expect(shown("dashboard.panels", layersOf({ mods: [hides], own: ordered })).slice(0, 2)).toEqual(["fleet", "merge-line"]);
  });

  it("never hides what cannot be hidden, whoever asks", () => {
    const asks = mod("greedy", '{"version":1,"rail":{"hidden":["settings","mods","overview","lessons"]},"job.tabs":{"hidden":["plan","overview","pulse"]},"dashboard.panels":{"hidden":["fleet"]}}');
    const layers = layersOf({ mods: [asks], own: { regions: {} } });
    expect(shown("rail", layers)).toEqual(expect.arrayContaining(["settings", "mods", "overview"]));
    expect(shown("rail", layers)).not.toContain("lessons");
    expect(shown("job.tabs", layers)).toEqual(expect.arrayContaining(["plan", "overview"]));
    expect(shown("job.tabs", layers)).not.toContain("pulse");
    expect(shown("dashboard.panels", layers)).toContain("fleet");
  });

  it("skips an id that is gone, and opens on the first entry that is shown when `first` is hidden", () => {
    const stale = mod("old", '{"version":1,"job.tabs":{"order":["removed-tab","record"],"hidden":["removed-tab","record"],"first":"record"}}');
    const resolved = resolveLayout("job.tabs", layersOf({ mods: [stale], own: { regions: {} } }));
    expect(resolved.all.map((one) => one.id)).not.toContain("removed-tab");
    expect(resolved.shown.map((one) => one.id)).not.toContain("record");
    expect(resolved.first).toBe("overview");
  });
});
