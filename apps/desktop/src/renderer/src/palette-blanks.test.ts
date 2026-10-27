// The steps a Commands mod's row asks before it runs: one a blank, in the order they first appear. A pull request, an issue
// or a repository is chosen from a list; any other blank is a word. Each step says which command it is for, and its act says
// Next until the last, then what the command does.

import type { CommandsFile } from "@armada/protocol";
import { describe, expect, test, vi } from "vitest";

import { askedOf, blankStep, firstStep, type BlankWorld } from "./palette-blanks";
import { rowsOf, type ModRow } from "./mod-commands";
import type { PaletteLinked } from "./palette-fixtures";

const linked = (repo: string, number: number, title: string): PaletteLinked => ({ repo, number, title, url: `https://forge.example/${repo}/${number}` }) as PaletteLinked;

function world(over: Partial<BlankWorld> = {}) {
  const run = vi.fn();
  const given: BlankWorld = {
    pulls: [linked("armada", 1899, "Plan windows"), linked("armada", 1901, "Palette acts")],
    issues: [linked("armada", 40, "Slow start")],
    repositories: [
      { root: "/Users/me/armada", label: "armada" },
      { root: "/Users/me/pocket", label: "Pocket" },
    ],
    picked: null,
    run,
    ...over,
  };
  return { given, run };
}

const rowOf = (command: CommandsFile["command"][number]): ModRow => rowsOf("m", { command: [command] })[0]!;

describe("what a row asks", () => {
  test("its blanks, in the order they first appear", () => {
    expect(askedOf(rowOf({ name: "R", prompt: "{focus} then {pr}", blanks: ["focus", "pr"] }), null)).toEqual(["focus", "pr"]);
  });

  test("a shell line also asks for a repository where none is picked, last", () => {
    const audit = rowOf({ name: "A", shell: "pnpm audit {pr}", blanks: ["pr"] });
    expect(askedOf(audit, null)).toEqual(["pr", "repo"]);
    expect(askedOf(audit, "/Users/me/armada")).toEqual(["pr"]);
  });

  test("a shell line whose blank is already a repository asks it once", () => {
    expect(askedOf(rowOf({ name: "A", shell: "pnpm -C {repo} audit", blanks: ["repo"] }), null)).toEqual(["repo"]);
  });

  test("a row that asks nothing has no first step", () => {
    const { given } = world({ picked: "/Users/me/armada" });
    expect(firstStep(rowOf({ name: "A", shell: "pnpm audit" }), given)).toBeUndefined();
    expect(firstStep(rowOf({ name: "U", url: "https://forge.example/" }), given)).toBeUndefined();
  });
});

describe("each step", () => {
  test("a pull request or an issue lists what can fill it, and a repository lists the repositories", () => {
    const { given } = world();
    const pr = firstStep(rowOf({ name: "Review", prompt: "Review {pr}", blanks: ["pr"] }), given)!;
    expect(pr.title).toBe("Pull request");
    expect(pr.entries.map((one) => one.label)).toEqual(["Plan windows", "Palette acts"]);
    const issue = firstStep(rowOf({ name: "Look", prompt: "Look at {issue}", blanks: ["issue"] }), given)!;
    expect(issue.title).toBe("Issue");
    expect(issue.entries.map((one) => one.label)).toEqual(["Slow start"]);
    const repo = firstStep(rowOf({ name: "Where", shell: "ls {repo}", blanks: ["repo"] }), given)!;
    expect(repo.title).toBe("Repository");
    expect(repo.entries.map((one) => one.label)).toEqual(["armada", "Pocket"]);
  });

  test("any other blank takes a word, titled from its name", () => {
    const { given } = world();
    const step = firstStep(rowOf({ name: "Review", prompt: "Look at {focus_area}", blanks: ["focus_area"] }), given)!;
    expect(step).toMatchObject({ title: "Focus area", word: true, entries: [], sections: [] });
  });

  test("every step carries the command's name", () => {
    const { given } = world();
    const row = rowOf({ name: "Security pass", prompt: "{pr} {issue} {focus}", blanks: ["pr", "issue", "focus"] });
    expect([0, 1, 2].map((at) => blankStep(row, at, {}, given).of)).toEqual(["Security pass", "Security pass", "Security pass"]);
  });

  test("the act says Next until the last step, then the act's verb", () => {
    const { given } = world();
    const verbs = (row: ModRow) => askedOf(row, given.picked).map((_, at) => blankStep(row, at, {}, given).act);
    expect(verbs(rowOf({ name: "P", prompt: "{pr} {focus}", blanks: ["pr", "focus"] }))).toEqual(["Next", "Start a Job"]);
    expect(verbs(rowOf({ name: "U", url: "https://forge.example/{pr}/{issue}", blanks: ["pr", "issue"] }))).toEqual(["Next", "Open"]);
    expect(verbs(rowOf({ name: "S", shell: "pnpm audit {pr}", blanks: ["pr"] }))).toEqual(["Next", "Run"]);
  });
});

describe("answering", () => {
  test("a choice that is not the last goes on to the next step, and runs nothing", () => {
    const { given, run } = world();
    const row = rowOf({ name: "Review", prompt: "Review {pr} for {focus}", blanks: ["pr", "focus"] });
    const next = firstStep(row, given)!.onChoose(firstStep(row, given)!.entries[0]!);
    expect(next).toMatchObject({ title: "Focus", of: "Review" });
    expect(run).not.toHaveBeenCalled();
  });

  test("the last answer runs the row with every blank filled: a pull request as #n and as the bare number", () => {
    const { given, run } = world();
    const row = rowOf({ name: "Review", prompt: "Review {pr} for {focus}", blanks: ["pr", "focus"] });
    const second = firstStep(row, given)!.onChoose(firstStep(row, given)!.entries[1]!)!;
    second.onChoose({ id: "word:auth", section: "word", label: "auth" });
    expect(run).toHaveBeenCalledWith(row, { pr: { text: "#1901", bare: "1901" }, focus: { text: "auth", bare: "auth" } });
  });

  test("a repository answer keeps its root, which a shell line runs in", () => {
    const { given, run } = world();
    const row = rowOf({ name: "Audit", shell: "pnpm audit" });
    const step = firstStep(row, given)!;
    expect(step).toMatchObject({ title: "Repository", act: "Run" });
    step.onChoose(step.entries[1]!);
    expect(run).toHaveBeenCalledWith(row, { repo: { text: "Pocket", bare: "Pocket", root: "/Users/me/pocket" } });
  });
});
