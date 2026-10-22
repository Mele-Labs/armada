import { expect, it } from "vitest";
import type { KitInventory, LoadoutItem, SetupItem } from "@armada/protocol";

import { skillActsByKit } from "./loadout-kit";

const HOME = "/Users/user/.harness";
const PLUGIN = `${HOME}/plugins/cache/productivity/1.0.0`;

const inventory = (skills: SetupItem[], plugins: SetupItem[] = []): KitInventory => ({
  harness: "Harness",
  home: HOME,
  present: true,
  kinds: [
    { kind: "skills", read: { what: "read", items: skills, unreadable: [] } },
    { kind: "plugins", read: { what: "read", items: plugins, unreadable: [] } },
  ],
});

const OWN: SetupItem = { name: "commit-message", source: `${HOME}/skills/commit-message` };
const BROUGHT: SetupItem = { name: "memory-management", source: `${PLUGIN}/skills/memory-management`, plugin: "productivity" };
const SUMMARISE: SetupItem = { name: "summarise", source: `${PLUGIN}/skills/summarise`, plugin: "productivity" };
const PRODUCTIVITY: SetupItem = { name: "productivity@toolshed", says: "version 1.0.0", source: PLUGIN, uninstall: "harness plugin uninstall productivity@toolshed" };

const skill = (name: string, source: LoadoutItem["source"], from?: string): LoadoutItem => ({ kind: "skill", name, source, ...(from === undefined ? {} : { from }) });

it("opens one of the person's own skills at its SKILL.md", () => {
  const acts = skillActsByKit([skill("commit-message", "kit")], inventory([OWN]));
  expect(acts["skill:kit::commit-message"]?.edits).toBe(`${HOME}/skills/commit-message/SKILL.md`);
});

it("opens a plugin's skill at its SKILL.md, by the name without the plugin's prefix", () => {
  const acts = skillActsByKit([skill("productivity:memory-management", "plugin", "productivity")], inventory([BROUGHT, SUMMARISE], [PRODUCTIVITY]));
  expect(acts["skill:plugin:productivity:productivity:memory-management"]?.edits).toBe(`${PLUGIN}/skills/memory-management/SKILL.md`);
});

it("offers nothing on a repository's, a built in or a managed skill, even one named as a skill Kit holds", () => {
  const items = [skill("commit-message", "repository"), skill("commit-message", "built_in"), skill("commit-message", "managed")];
  expect(skillActsByKit(items, inventory([OWN]))).toEqual({});
});

it("offers nothing where the inventory names the skill twice, or not at all", () => {
  expect(skillActsByKit([skill("commit-message", "kit")], inventory([OWN, { ...OWN, source: `${HOME}/skills/nested/commit-message` }]))).toEqual({});
  expect(skillActsByKit([skill("pdf", "kit")], inventory([OWN]))).toEqual({});
});
