// Which editor opens a path, and how `editor.command`'s placeholders are filled. `editor.ts`.

import { describe, expect, it } from "vitest";

import { chooseEditor, withFile } from "./editor";

describe("chooseEditor", () => {
  it("takes editor.command over $VISUAL and $EDITOR", () => {
    expect(chooseEditor({ VISUAL: "vim", EDITOR: "nano" }, "code -g {file}:{line}")).toEqual({
      command: "code",
      args: ["-g", "{file}:{line}"],
      from: "editor.command",
    });
  });

  it("falls to $VISUAL, then $EDITOR, where editor.command is empty", () => {
    expect(chooseEditor({ VISUAL: "vim", EDITOR: "nano" }, "  ")?.from).toBe("$VISUAL");
    expect(chooseEditor({ EDITOR: "code -w" }, "")).toEqual({ command: "code", args: ["-w"], from: "$EDITOR" });
    expect(chooseEditor({}, "")).toBeNull();
  });
});

describe("withFile", () => {
  it("fills {file} and {line} inside the word they are written in", () => {
    expect(withFile(["-g", "{file}:{line}"], "/repo/a.ts", 12)).toEqual(["-g", "/repo/a.ts:12"]);
  });

  it("puts the path last where nothing names {file}, as $EDITOR always has", () => {
    expect(withFile(["-w"], "/repo/a.ts")).toEqual(["-w", "/repo/a.ts"]);
  });

  it("reads a line it was not given as the first", () => {
    expect(withFile(["{file}:{line}"], "/repo/a.ts")).toEqual(["/repo/a.ts:1"]);
  });
});
