// What a Commands mod's file becomes in the palette: a row id, the words and the key it is searched and run by, and the
// blanks its text is filled from. A file's key is allowed only as a person's own would be, and one he recorded wins.

import { describe, expect, test } from "vitest";
import type { CommandsFile } from "@armada/protocol";

import { displayOf, fill, keysOf, rowsOf, slugOf } from "./mod-commands";

const FILE: CommandsFile = {
  command: [
    { name: "Security pass on a pull request", aliases: ["sec", "audit"], hotkey: "⌥⌘S", prompt: "Review {pr} for {focus} problems", blanks: ["pr", "focus"] },
    { name: "Open pull request on the forge", url: "https://forge.example/pull/{pr}", blanks: ["pr"] },
    { name: "Audit dependencies", shell: "pnpm audit" },
    { name: "Triage the board", chain: ["nav:reviews", "nav:projects"] },
  ],
};

describe("a row's id", () => {
  test("is the mod and the slug of the name, which the palette draws as cmd:<mod>:<slug>", () => {
    expect(rowsOf("team-commands", FILE).map((one) => `cmd:${one.id}`)).toEqual([
      "cmd:team-commands:security-pass-on-a-pull-request",
      "cmd:team-commands:open-pull-request-on-the-forge",
      "cmd:team-commands:audit-dependencies",
      "cmd:team-commands:triage-the-board",
    ]);
  });

  test("slugs lower-case the name and make each run of anything but letters and digits one hyphen, none at either end", () => {
    expect(slugOf("  Run: Fix—it (now)!  ")).toBe("run-fix-it-now");
    expect(slugOf("Überprüfen 2")).toBe("überprüfen-2");
  });
});

describe("what the palette draws of a row", () => {
  const [prompt, url, shell, chain] = rowsOf("m", FILE);

  test("its kind is the act it has, and its alias is searchable", () => {
    expect(displayOf(prompt!, undefined)).toMatchObject({ id: "m:security-pass-on-a-pull-request", label: "Security pass on a pull request", kind: "prompt", aliases: ["sec", "audit"] });
    expect(displayOf(url!, undefined).kind).toBe("url");
    expect(displayOf(shell!, undefined).kind).toBe("shell");
    expect(displayOf(chain!, undefined).kind).toBe("chain");
    expect(displayOf(shell!, undefined)).not.toHaveProperty("aliases");
  });

  test("a row shows the key it was allowed, not the one the file names", () => {
    expect(displayOf(prompt!, "⇧⌘S").hotkey).toBe("⇧⌘S");
    expect(displayOf(prompt!, undefined)).not.toHaveProperty("hotkey");
  });
});

describe("the keys the files name", () => {
  const keyed = (hotkey: string | undefined, name = "Run") => rowsOf("m", { command: [{ name, ...(hotkey === undefined ? {} : { hotkey }), shell: "true" }] });

  test("an allowed key is kept, by the row's palette id", () => {
    expect(keysOf(rowsOf("team-commands", FILE), {})).toEqual({ "cmd:team-commands:security-pass-on-a-pull-request": "⌥⌘S" });
  });

  test("a key the shared rules refuse is dropped: no ⌘ or ⌃, ⌘1 to ⌘9, an act's, the system's", () => {
    for (const refused of ["⇧J", "⌘1", "⌘K", "⌘C", "not a key"]) expect(keysOf(keyed(refused), {})).toEqual({});
  });

  test("a key another row already has is dropped, the first row in file order keeping it", () => {
    const rows = rowsOf("m", {
      command: [
        { name: "First", hotkey: "⇧⌘J", shell: "true" },
        { name: "Second", hotkey: "⇧⌘J", shell: "true" },
      ],
    });
    expect(keysOf(rows, {})).toEqual({ "cmd:m:first": "⇧⌘J" });
  });

  const two = rowsOf("m", {
    command: [
      { name: "First", hotkey: "⇧⌘J", shell: "true" },
      { name: "Second", hotkey: "⇧⌘L", shell: "true" },
    ],
  });

  test("a key he recorded for a row wins over the file's: the file's is not used for that row", () => {
    expect(keysOf(two, { "cmd:m:second": "⇧⌘M" })).toEqual({ "cmd:m:first": "⇧⌘J" });
  });

  test("a key he recorded holds its place first: a file's key the same is dropped", () => {
    expect(keysOf(two, { "cmd:elsewhere": "⇧⌘J" })).toEqual({ "cmd:m:second": "⇧⌘L" });
  });
});

describe("filling the blanks", () => {
  test("each blank is replaced by what it was filled with, and one unfilled is left", () => {
    expect(fill("Review {pr} for {focus} problems", { pr: "#12", focus: "auth" })).toBe("Review #12 for auth problems");
    expect(fill("Review {pr} for {focus}", { pr: "#12" })).toBe("Review #12 for {focus}");
  });

  test("a brace after a $ is shell text and stays", () => {
    expect(fill("echo ${home} {word}", { word: "x", home: "no" })).toBe("echo ${home} x");
  });
});
