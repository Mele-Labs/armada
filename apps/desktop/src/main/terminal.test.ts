// Which terminal opens a worktree, from `terminal.app` and `terminal.command`. `terminal.ts`.

import { describe, expect, it } from "vitest";

import { terminalFor } from "./terminal";

const DIR = "/repo/.armada/worktrees/01JOB";

describe("terminalFor", () => {
  it("opens a named app on a Mac through `open`, with the directory as its own argument", async () => {
    expect(await terminalFor("Terminal", "", DIR, {}, "darwin")).toEqual({ ok: true, program: "/usr/bin/open", argv: ["-a", "Terminal", DIR] });
    expect(await terminalFor("WezTerm", "", DIR, {}, "darwin")).toEqual({
      ok: true,
      program: "/usr/bin/open",
      argv: ["-na", "WezTerm", "--args", "start", "--cwd", DIR],
    });
  });

  it("reads an unset app as Terminal", async () => {
    expect(await terminalFor("", "", DIR, {}, "darwin")).toMatchObject({ ok: true, argv: ["-a", "Terminal", DIR] });
  });

  it("fills {dir} in a custom command, and never hands it to a shell", async () => {
    const launch = await terminalFor("Custom", "sh -c {dir};echo", DIR, { PATH: "/bin:/usr/bin" }, "linux");
    expect(launch).toMatchObject({ ok: true, argv: ["-c", `${DIR};echo`] });
  });

  it("says which setting is wrong where it cannot open one", async () => {
    expect(await terminalFor("Custom", "", DIR, {}, "darwin")).toMatchObject({ ok: false, detail: expect.stringContaining("terminal.command is empty") });
    expect(await terminalFor("Custom", "no-such-terminal {dir}", DIR, { PATH: "/bin" }, "darwin")).toMatchObject({
      ok: false,
      detail: "terminal.command names no-such-terminal, and nothing on PATH is called that.",
    });
    expect(await terminalFor("Ghostty", "", DIR, {}, "linux")).toMatchObject({ ok: false, detail: expect.stringContaining("Choose Custom") });
  });
});
