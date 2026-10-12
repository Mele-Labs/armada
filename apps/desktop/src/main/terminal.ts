// Which terminal a worktree is opened in: `terminal.app` in settings.json, or `terminal.command`
// when that is Custom. `editor.ts`' rules hold: an argv and never a shell, the program found before
// anything starts, and a failure that names the setting it came from.

import { startDetached, whereIs } from "./editor";

/** The terminals `terminal.app` offers, besides Custom, and how each is asked to open a directory. */
const APPS: Record<string, (dir: string) => string[]> = {
  // `open -a` hands the directory to the app, which opens a window there.
  Terminal: (dir) => ["-a", "Terminal", dir],
  iTerm: (dir) => ["-a", "iTerm", dir],
  Warp: (dir) => ["-a", "Warp", dir],
  // These two take no directory through `open`; each is started with its own flag instead.
  Ghostty: (dir) => ["-na", "Ghostty", "--args", `--working-directory=${dir}`],
  WezTerm: (dir) => ["-na", "WezTerm", "--args", "start", "--cwd", dir],
};

/** A program and its arguments, or the sentence that says why there is none. */
export type Launch = { ok: true; program: string; argv: string[] } | { ok: false; detail: string };

/**
 * How to open `dir` in the terminal the settings name. `platform` is a parameter so a test on
 * another system can ask what a Mac would do.
 */
export async function terminalFor(
  app: string,
  command: string,
  dir: string,
  env: NodeJS.ProcessEnv,
  platform: NodeJS.Platform = process.platform,
): Promise<Launch> {
  if (app === "Custom") {
    const words = command.trim().split(/\s+/).filter((word) => word !== "");
    const [program, ...args] = words;
    if (program === undefined) {
      return { ok: false, detail: "terminal.app is Custom and terminal.command is empty. Set terminal.command in Settings." };
    }
    const found = await whereIs(program, env);
    if (found === null) return { ok: false, detail: `terminal.command names ${program}, and nothing on PATH is called that.` };
    const filled = args.map((word) => word.replaceAll("{dir}", dir));
    return { ok: true, program: found, argv: args.some((word) => word.includes("{dir}")) ? filled : [...filled, dir] };
  }
  const argv = APPS[app === "" ? "Terminal" : app];
  if (argv === undefined) return { ok: false, detail: `terminal.app names ${app}, which Bridge does not know how to open.` };
  if (platform !== "darwin") {
    return { ok: false, detail: `${app} is opened through macOS, and this is not a Mac. Choose Custom and set terminal.command.` };
  }
  return { ok: true, program: "/usr/bin/open", argv: argv(dir) };
}

/** Open `dir` in the terminal, or say why not. Detached: the terminal outlives the press. */
export async function openTerminal(app: string, command: string, dir: string, env: NodeJS.ProcessEnv): Promise<Launch> {
  const launch = await terminalFor(app, command, dir, env);
  if (launch.ok) startDetached(launch.program, launch.argv);
  return launch;
}
