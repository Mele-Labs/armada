// Kit on a second machine signed in to the same GitHub account as the first, against the shared home
// the two machines read: a private repository carrying Kit, workflows, the allowlist, a repository's
// memory and the global instructions. Workflows match, the allowlist differs by two commands, Memory
// has entries only in the home, the global instructions differ, and one server needs its key here.
// The walk `shared-home` plays it.

import type { KitInventory } from "@armada/protocol";
import type { SharedRow } from "@armada/components";

import { KIT_INVENTORY, manifesting } from "../manifest-fake";
import type { Scenario } from "../moment";

const SERVERS = "/Users/user/.agent.json";

/** Fifty more skills, so a tile's list is long enough to scroll. */
const GENERATED_NAMES = Array.from({ length: 50 }, (_, at) => `skill-${String(at + 1).padStart(2, "0")}`);
const GENERATED = GENERATED_NAMES.map((name) => ({ name, source: `/Users/user/.agent/skills/${name}` }));

const INVENTORY: KitInventory = {
  ...KIT_INVENTORY,
  harness: "Claude",
  kinds: KIT_INVENTORY.kinds.map((one) => {
    if (one.kind === "skills" && one.read.what === "read") return { ...one, read: { ...one.read, items: [...one.read.items, ...GENERATED], unreadable: [] } };
    if (one.kind === "mcp_servers") {
      return {
        kind: "mcp_servers",
        read: {
          what: "read",
          items: [
            { name: "gitnexus", says: "gitnexus-mcp", source: SERVERS },
            { name: "linear", says: "mcp.linear.app", source: SERVERS },
          ],
          unreadable: [],
        },
      };
    }
    if (one.kind === "allowlist") {
      return {
        kind: "allowlist",
        read: {
          what: "read",
          items: [
            { name: "cargo check", source: "written by hand" },
            { name: "pnpm exec tsc", source: "always allow" },
            { name: "gh pr view", source: "retro item 7" },
            { name: "git log --oneline", source: "always allow" },
          ],
          unreadable: [],
        },
      };
    }
    return one;
  }),
};

const HERE = [
  "# Global Instructions",
  "",
  "## Core Directive",
  "Direct, highly technical. Raw signal over polish.",
  "",
  "## Communication",
  "* Begin with the solution. No openers.",
  "* Prefer ASCII diagrams and tables to prose.",
  "* Questions go last.",
  "",
  "## Coding",
  "* State assumptions before coding.",
  "* Touch only what the task needs.",
  "* Define success, then loop until it is met.",
].join("\n");

const THERE = [
  "# Global Instructions",
  "",
  "## Core Directive",
  "Direct, highly technical. Raw signal over polish.",
  "",
  "## Communication",
  "* Begin with the solution. No openers.",
  "* Prefer ASCII diagrams and tables to prose.",
  "* Questions go last, in the prompt.",
  "* Run the humanizer over every message.",
  "",
  "## Coding",
  "* State assumptions before coding.",
  "* Touch only what the task needs.",
  "* Define success, then loop until it is met.",
  "* Verify by looking when the work is visual.",
].join("\n");

const row = (kind: string, name: string, state: SharedRow["state"], more: Partial<SharedRow> = {}): SharedRow => ({ kind, name, state, ...more });

const ROWS: SharedRow[] = [
  row("skills", "humanizer", "same"),
  row("skills", "impact-analysis", "same"),
  row("skills", "release-notes", "here"),
  row("skills", "deploy", "here"),
  ...GENERATED_NAMES.map((name) => row("skills", name, "same")),
  row("plugins", "code-simplifier@official", "same"),
  row("global_instructions", "AGENTS.md", "differs", {
    says: "21 lines, opening # Global Instructions",
    source: "/Users/user/.agent/AGENTS.md",
    here: HERE,
    there: THERE,
  }),
  row("workflows", "bug", "same"),
  row("workflows", "feature", "same"),
  row("workflows", "refactor", "same"),
  row("mcp_servers", "gitnexus", "same"),
  row("mcp_servers", "linear", "same", { keyHere: true }),
  row("allowlist", "cargo check", "same"),
  row("allowlist", "pnpm exec tsc", "same"),
  row("allowlist", "gh pr view", "same"),
  row("allowlist", "git log --oneline", "here"),
  row("allowlist", "gh run view", "home", { source: "always allow" }),
  row("memory", "Walks, not pictures", "same", { source: "armada", says: "Show a visual change as a walk link, never a screenshot" }),
  row("memory", "Subagents on Sonnet", "same", { source: "armada", says: "Pass model sonnet on every Agent call" }),
  row("memory", "Never generate load", "home", { source: "armada", says: "No burners, no whole suite runs" }),
  row("memory", "Two agents at once", "home", { source: "armada", says: "A session runs at most two children" }),
  row("memory", "Icons by registry name", "home", { source: "armada", says: "Offer registry glyphs by name, never emoji" }),
  row("memory", "Questions at the end", "home", { source: "armada", says: "Ask with the prompt at the end of the reply" }),
];

/** The row a Sync leaves on this machine, with the sentence Fleet tells for it. */
const STAYS_HERE = [{ kind: "skills", name: "deploy", why: "deploy stays on this machine: an address at git.acme.dev carries a user and password" }];

function build(): Scenario {
  const base = manifesting({ kitInventory: INVENTORY });
  return {
    ...base,
    name: "shared-home",
    says: "Kit on a second machine against the shared home: each row's standing, and the two versions of a row that differs",
    draft: { ...(base.draft ?? {}), sharedHome: { account: "nickmele", repository: "armada-home", rows: ROWS, staysHere: STAYS_HERE } },
  };
}

export const sharedHome: Scenario = build();
