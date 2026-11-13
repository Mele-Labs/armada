// Start Fleet, and reopen this Bridge, from the window: the two acts a Bridge offers when the Fleet
// beside it is not running, or when it is the stale side of a mismatch.
//
// **Start Fleet is `armada fleet start` from the kept build `builds/current` names**, or the `armada` on
// PATH where there is none. It is the CLI's act and not a `launchctl` call from here, so a Bridge and the
// terminal start Fleet the same way, and the three cases (not loaded, stopped, running) are decided once.
// The CLI reads `ARMADA_FLEET_LABEL` from the environment this process passes it.
//
// **Reopen Armada restarts this Bridge's own launchd job**, `<fleet label>.bridge`, whose launcher opens
// whichever kept build `builds/current` names. The call is detached and unref'd: launchd stops this
// process as part of it, and a child of it would go too.

import { execFile, spawn } from "node:child_process";
import { existsSync, realpathSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";

import type { BridgeReopen, FleetStart, KeptBuilds } from "@armada/protocol";

import { labelOf } from "./restart-fleet";

/** What a command answered. */
export type Exec = (
  file: string,
  args: string[],
) => Promise<{
  code: number;
  stderr: string;
}>;

export const exec: Exec = (file, args) =>
  new Promise((resolve) => {
    execFile(file, args, (error, _stdout, stderr) => {
      const code = error === null ? 0 : typeof error.code === "number" ? error.code : 1;
      resolve({
        code,
        stderr: stderr === "" && error !== null ? error.message : stderr,
      });
    });
  });

/** The `armada` of the build `current` names under `builds`, or the one on PATH. */
export function armadaIn(builds: string | null): string {
  const kept = builds === null ? null : join(builds, "current", "armada");
  return kept !== null && existsSync(kept) ? kept : "armada";
}

/** Start Fleet where launchd holds it and it is not running. Says nothing of whether it answers yet. */
export async function startFleet(
  builds: string | null,
  run: Exec = exec,
  armada: string = armadaIn(builds),
): Promise<FleetStart> {
  const ran = await run(armada, [
    "fleet",
    "start",
  ]);
  if (ran.code === 0)
    return {
      ok: true,
    };
  return {
    ok: false,
    why: "refused",
    detail: ran.stderr.trim() || `${armada} fleet start exited ${ran.code}`,
  };
}

/** Starts a command that outlives this process, and says whether it started. */
export type Detach = (file: string, args: string[]) => Promise<string | null>;

export const detach: Detach = (file, args) =>
  new Promise((resolve) => {
    const child = spawn(file, args, {
      detached: true,
      stdio: "ignore",
    });
    child.once("error", (error) => resolve(error.message));
    child.once("spawn", () => {
      child.unref();
      resolve(null);
    });
  });

/** Restart this Bridge's launchd job, which reopens it on `builds/current`. */
export async function reopenBridge(
  start: Detach = detach,
  env: NodeJS.ProcessEnv = process.env,
  uid: number = process.getuid?.() ?? 0,
): Promise<BridgeReopen> {
  const would = await start("launchctl", [
    "kickstart",
    "-k",
    `gui/${uid}/${labelOf(env)}.bridge`,
  ]);
  return would === null
    ? {
        ok: true,
      }
    : {
        ok: false,
        detail: would,
      };
}

function realpath(path: string): string | null {
  try {
    return realpathSync(path);
  } catch {
    return null;
  }
}

/** The name of the kept build folder `folder` is, or null where it is not directly under `builds`. */
function nameUnder(builds: string, folder: string | null): string | null {
  return folder !== null && dirname(folder) === builds ? basename(folder) : null;
}

/**
 * The kept build this Bridge runs from, and the one `builds/current` names. **Both resolved through
 * links**, so `current` reads as the build it points at. `appPath` is the app folder, `<name>/bridge`.
 */
export function keptBuildsOf(appPath: string, builds: string | null): KeptBuilds {
  const root = builds === null ? null : realpath(builds);
  if (builds === null || root === null)
    return {
      bridge: null,
      current: null,
    };
  return {
    bridge: nameUnder(root, realpath(join(resolve(appPath), ".."))),
    current: nameUnder(root, realpath(join(builds, "current"))),
  };
}
