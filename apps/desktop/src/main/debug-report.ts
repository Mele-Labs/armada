// Copy debug report's text from the two places it can come from: Fleet over the connection, and where Fleet is down or
// does not answer, `armada report` run here. Read-only both ways: the report is names, versions and states, and nothing is sent.
//
// **The binary is the one launchd starts Fleet with**, which `scripts/restart` loads and `restart-fleet.ts` already asks. Bridge
// starts no Fleet and holds no path of its own, and a bare `armada` off the PATH of a packaged app is not the build that runs.

import { execFile } from "node:child_process";
import { access, constants } from "node:fs/promises";
import { homedir } from "node:os";
import { promisify } from "node:util";

import type { DebugReport, Outcome } from "@armada/protocol";
import type { DebugReportRead } from "../shared/api/debug-report";
import { ask } from "./request";
import { labelOf, launchctl, type Run } from "./restart-fleet";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

/** How long `armada report` has. It reads a log file and asks Fleet once, so a longer wait is a hang. */
export const REPORT_MS = 15_000;

/** The path for a window whose selected repository is `root`, or none. */
export function debugReportPath(root: string | null): string {
  return root === null ? "/debug_report" : `/debug_report?root=${encodeURIComponent(root)}`;
}

export class DebugReportHost {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  async get(root: string | null): Promise<DebugReportRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "GET", debugReportPath(root));
    return answer.ok === true ? { ok: true, value: answer.body as DebugReport } : { ok: false, outcome: answer.outcome };
  }
}

/** `execFile` as this file uses it, so a test hands in its own. */
export type Exec = (file: string, args: string[], options: { cwd: string; timeout: number }) => Promise<{ stdout: string }>;

const exec: Exec = (file, args, options) => promisify(execFile)(file, args, { ...options, encoding: "utf8" });

const runnable = (path: string): Promise<boolean> =>
  access(path, constants.X_OK).then(
    () => true,
    () => false,
  );

/**
 * The `armada` launchd runs Fleet from, or the one inside the app where launchd holds no job, or `null`.
 * Only a path that exists and runs is returned.
 */
export async function locateArmada(
  bundled: string | null,
  launchd: Run = launchctl,
  env: NodeJS.ProcessEnv = process.env,
  uid: number = process.getuid?.() ?? 0,
  runs: (path: string) => Promise<boolean> = runnable,
): Promise<string | null> {
  const held = await launchd(["print", `gui/${uid}/${labelOf(env)}`]);
  const program = held.code === 0 ? /^\s*program = (.+)$/m.exec(held.stdout)?.[1]?.trim() : undefined;
  if (program !== undefined && (await runs(program))) return program;
  return bundled !== null && (await runs(bundled)) ? bundled : null;
}

/**
 * What `armada report` prints, run in `cwd` with no shell. **Rejects with the sentence the sheet shows**: no binary, no answer
 * within `REPORT_MS`, a failed run, or nothing printed.
 */
export async function armadaReport(program: string | null, cwd: string, run: Exec = exec): Promise<string> {
  if (program === null) throw new Error("no armada binary was found");
  let stdout: string;
  try {
    ({ stdout } = await run(program, ["report"], { cwd, timeout: REPORT_MS }));
  } catch (cause) {
    const failure = cause as { killed?: boolean; stderr?: string; message?: string };
    if (failure.killed === true) throw new Error(`armada report did not finish in ${REPORT_MS / 1000} seconds`);
    throw new Error(`armada report failed: ${failure.stderr?.trim().split("\n")[0] || failure.message || "it failed"}`);
  }
  const text = stdout.trimEnd();
  if (text === "") throw new Error("armada report printed nothing");
  return text;
}

/** A folder the renderer names is used only as the working folder of a read-only command, and only where it is absolute. */
export function rootOf(root: unknown): string | null {
  return typeof root === "string" && root.startsWith("/") ? root : null;
}

/** The folder `armada report` runs in: the selected repository, or home. */
export const whereToRun = (root: string | null): string => root ?? homedir();
