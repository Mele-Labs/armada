// The armada-mods skill teaches a session the regions and ids a layout may name. It is prose, so
// this holds it to the registry file Fleet checks: every region and id is in the skill's table, and
// the two examples it shows pass the rules Bridge applies.

import { expect, it } from "vitest";

import registry from "../../../shell/layout-registry.json";
import { parseLayout } from "../../../shell/src/layout";

// Typed by hand, as `catalogue.ts` does: Vite's client types are not loaded here.
type Globbing = ImportMeta & { glob(pattern: string, options: { eager: true; query: string; import: string }): Record<string, string> };
const FILES = (import.meta as Globbing).glob("../../../../.claude/skills/armada-mods/SKILL.md", { eager: true, query: "?raw", import: "default" });
const skill = Object.values(FILES)[0] ?? "";

it("lists every region and id the registry file ships, and each example passes", () => {
  expect(skill).toContain("# Making a layout mod");
  for (const [region, shipped] of Object.entries(registry.regions)) {
    const line = skill.split("\n").find((one) => one.startsWith(`| \`${region}\``));
    expect(line, region).toBeDefined();
    for (const entry of shipped.entries) expect(line, `${region} ${entry.id}`).toContain(`\`${entry.id}\``);
  }
  const examples = [...skill.matchAll(/```json\n([\s\S]*?)```/g)].map((one) => one[1]!);
  expect(examples).toHaveLength(2);
  for (const text of examples) expect(parseLayout(text).problems).toEqual([]);
});
