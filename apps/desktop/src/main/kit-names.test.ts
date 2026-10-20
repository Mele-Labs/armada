import { describe, expect, it } from "vitest";
import type { KitInventory } from "@armada/protocol";

import { namesKitPath } from "./kit-names";

const HOME = "/Users/user/.agent";
const setup: KitInventory = {
  harness: "Agent",
  home: HOME,
  present: true,
  kinds: [
    { kind: "skills", read: { what: "read", unreadable: [], items: [{ name: "a", source: `${HOME}/skills/a` }, { name: "rel", source: "skills/rel" }] } },
    { kind: "sub_agents", read: { what: "read", unreadable: [], items: [{ name: "r", source: `${HOME}/agents/r.md` }] } },
    { kind: "plugins", read: { what: "read", unreadable: [], items: [{ name: "p@m", source: `${HOME}/plugins/p` }] } },
    { kind: "allowlist", read: { what: "read", unreadable: [], items: [{ name: "x", source: `${HOME}/skills/b/SKILL.md` }] } },
  ],
};

describe("a skill folder's file", () => {
  it("is named by its folder, absolute or read under the home", () => {
    expect(namesKitPath(setup, `${HOME}/skills/a/SKILL.md`)).toBe(true);
    expect(namesKitPath(setup, `${HOME}/skills/rel/SKILL.md`)).toBe(true);
  });

  it("is only the SKILL.md, and only for a skill", () => {
    expect(namesKitPath(setup, `${HOME}/skills/a/other.md`)).toBe(false);
    expect(namesKitPath(setup, `${HOME}/agents/r.md/SKILL.md`)).toBe(false);
    expect(namesKitPath(setup, `${HOME}/plugins/p/SKILL.md`)).toBe(false);
  });

  it("is not named by the allowlist", () => {
    expect(namesKitPath(setup, `${HOME}/skills/b/SKILL.md`)).toBe(false);
  });
});
