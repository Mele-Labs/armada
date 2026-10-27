// What a Commands mod's row does when it runs: a prompt fills the dispatch sheet, an address is filled and encoded, a shell line
// goes to a side panel with a pull request as its bare number, and a chain runs the rows it names in order. A row run without the
// blanks it asks for opens the palette on its first step instead.

import type { CommandsFile } from "@armada/protocol";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import { rowsOf, type ModRow } from "./mod-commands";
import { setKnownRows } from "./palette-known";
import { runModRow, type Filled, type RowWorld } from "./palette-rows";

const MOD = "m";
const FILE: CommandsFile = {
  command: [
    { name: "Review", prompt: "Review {pr} for {focus} problems", blanks: ["pr", "focus"] },
    { name: "Search", url: "https://forge.example/search?q={focus}&pr={pr}", blanks: ["focus", "pr"] },
    { name: "Test a pull request", shell: "pnpm test --pr {pr}", blanks: ["pr"] },
    { name: "Audit", shell: "pnpm audit" },
    { name: "Triage", chain: ["nav:reviews", "cmd:m:audit-quietly"] },
    { name: "Audit quietly", prompt: "Audit the tree" },
    { name: "Ask first", chain: ["cmd:m:review", "nav:projects"] },
    { name: "Loop a", chain: ["cmd:m:loop-b"] },
    { name: "Loop b", chain: ["cmd:m:loop-a"] },
    { name: "Missing", chain: ["nav:reviews", "cmd:m:nowhere", "nav:projects"] },
    { name: "Dormant", chain: ["nav:reviews", "nav:stopped", "nav:projects"] },
  ],
};
const ROWS = rowsOf(MOD, FILE);
const row = (name: string): ModRow => ROWS.find((one) => one.command.name === name)!;

const filled = (over: Record<string, string>): Filled => Object.fromEntries(Object.entries(over).map(([name, text]) => [name, { text, bare: text.replace(/^#/, "") }]));

/** A world that records what was done, and runs a chain's rows as the palette does: a mod's by its own run, any other by its id. */
function world(over: Partial<RowWorld> = {}) {
  const log: string[] = [];
  const given = {
    modRows: ROWS,
    picked: null,
    repositories: [
      { root: "/Users/user/armada", label: "armada" },
      { root: "/Users/user/pocket", label: "Pocket" },
    ],
    startJob: (words: string) => log.push(`job:${words}`),
    openLink: (address: string) => (log.push(`link:${address}`), true),
    runShell: (command: string, dir: string, repository: string) => (log.push(`shell:${command}@${dir}(${repository})`), true),
    openAt: (one: ModRow) => log.push(`ask:${one.command.name}`),
    tell: (sentence: string) => log.push(`told:${sentence}`),
    runRow: (id: string) => {
      const mod = ROWS.find((one) => `cmd:${one.id}` === id);
      if (mod === undefined) log.push(`ran:${id}`);
      else runModRow(mod, {}, given as RowWorld);
    },
    ...over,
  } as unknown as RowWorld;
  return { given, log };
}

beforeEach(() => {
  setKnownRows([
    { id: "nav:reviews", label: "Reviews", kind: "Page", keyable: true, dormant: false },
    { id: "nav:projects", label: "Projects", kind: "Page", keyable: true, dormant: false },
    { id: "nav:stopped", label: "Stopped", kind: "Page", keyable: true, dormant: true },
    ...ROWS.map((one) => ({ id: `cmd:${one.id}`, label: one.command.name, kind: "Command", keyable: true, dormant: false })),
  ]);
});
afterEach(() => {
  setKnownRows([]);
  vi.unstubAllGlobals();
});

describe("a prompt", () => {
  test("fills the dispatch sheet with the words, each blank filled as the person saw it", () => {
    const { given, log } = world();
    runModRow(row("Review"), filled({ pr: "#12", focus: "auth" }), given);
    expect(log).toEqual(["job:Review #12 for auth problems"]);
  });

  test("with no blanks, starts at once", () => {
    const { given, log } = world();
    runModRow(row("Audit quietly"), {}, given);
    expect(log).toEqual(["job:Audit the tree"]);
  });
});

describe("an address", () => {
  test("is filled with each blank encoded, a pull request as its bare number", () => {
    const { given, log } = world();
    runModRow(row("Search"), filled({ focus: "a b&c/d", pr: "#7" }), given);
    expect(log).toEqual(["link:https://forge.example/search?q=a%20b%26c%2Fd&pr=7"]);
  });

  test("an address Bridge does not open itself goes to the browser, and one that did not open is told", async () => {
    const opened: string[] = [];
    vi.stubGlobal("window", { armada: { openLink: async (address: string) => (opened.push(address), { ok: false }) } });
    const { given, log } = world({ openLink: () => false });
    runModRow(row("Search"), filled({ focus: "x", pr: "#7" }), given);
    await vi.waitFor(() => expect(log).toEqual(["told:Search: https://forge.example/search?q=x&pr=7 did not open"]));
    expect(opened).toEqual(["https://forge.example/search?q=x&pr=7"]);
  });
});

describe("a shell line", () => {
  test("is typed with a pull request as its bare number, in the repository picked, and opens the run panel", () => {
    const { given, log } = world({ picked: "/Users/user/armada" });
    runModRow(row("Test a pull request"), filled({ pr: "#12" }), given);
    expect(log).toEqual(["shell:pnpm test --pr 12@/Users/user/armada(armada)"]);
  });

  test("runs in the repository the person was asked for where none is picked", () => {
    const { given, log } = world();
    runModRow(row("Audit"), { repo: { text: "Pocket", bare: "Pocket", root: "/Users/user/pocket" } }, given);
    expect(log).toEqual(["shell:pnpm audit@/Users/user/pocket(Pocket)"]);
  });

  test("with no repository picked or given, asks for one rather than run anywhere", () => {
    const { given, log } = world();
    runModRow(row("Audit"), {}, given);
    expect(log).toEqual(["ask:Audit"]);
  });

  test("says so where no side panel is hosted", () => {
    const { given, log } = world({ picked: "/Users/user/armada", runShell: () => false });
    runModRow(row("Audit"), {}, given);
    expect(log).toEqual(["told:Audit: no side panel to show it in"]);
  });
});

test("a row run without a blank it asks for opens the palette on its first step and does nothing else", () => {
  const { given, log } = world();
  runModRow(row("Review"), filled({ pr: "#12" }), given);
  expect(log).toEqual(["ask:Review"]);
});

describe("a chain", () => {
  test("runs the rows it names in order, a mod's row as its own and any other by its id", () => {
    const { given, log } = world();
    runModRow(row("Triage"), {}, given);
    expect(log).toEqual(["ran:nav:reviews", "job:Audit the tree"]);
  });

  test("stops at an id with no row, naming it, and runs nothing after", () => {
    const { given, log } = world();
    runModRow(row("Missing"), {}, given);
    expect(log).toEqual(["ran:nav:reviews", "told:Missing stopped: no row cmd:m:nowhere"]);
  });

  test("stops at a row that cannot run, naming it, and runs nothing after", () => {
    const { given, log } = world();
    runModRow(row("Dormant"), {}, given);
    expect(log).toEqual(["ran:nav:reviews", "told:Dormant stopped at Stopped: it cannot run"]);
  });

  test("stops where it comes back to a row it is running", () => {
    const { given, log } = world();
    runModRow(row("Loop a"), {}, given);
    expect(log).toEqual(["told:Loop a chains back to itself"]);
  });

  test("a chain that ran, or stopped, can run again: its place in the loop guard is given back", () => {
    const { given, log } = world();
    runModRow(row("Triage"), {}, given);
    runModRow(row("Missing"), {}, given);
    runModRow(row("Triage"), {}, given);
    expect(log).toEqual(["ran:nav:reviews", "job:Audit the tree", "ran:nav:reviews", "told:Missing stopped: no row cmd:m:nowhere", "ran:nav:reviews", "job:Audit the tree"]);
  });

  test("reaching a row that asks for a blank opens the palette on its first step, and the chain ends there", () => {
    const { given, log } = world();
    runModRow(row("Ask first"), {}, given);
    expect(log).toEqual(["ask:Review"]);
  });
});
