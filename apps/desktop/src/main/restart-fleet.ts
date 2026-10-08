// Restart Fleet from the window, by asking launchd to restart the job that runs it.
//
// **A `launchctl` call from Bridge and not an API command**, which is what
// `docs/concepts/fleet.md` decided: the process being restarted cannot serve
// the request, and a Fleet too old to have a route for it is the one this is
// most needed against. Nothing crosses the wire, so it moves no protocol ID.
//
// **It restarts onto whatever `armada` is installed.** Bridge cannot build or
// install one. If that binary is not this Bridge's build the two still differ
// afterwards, and the window says so.
//
// **Working Drones outlive the restart and are adopted at boot**, the same as
// `scripts/restart --adopt`: Fleet is SIGTERMed and nothing here touches a
// Drone. The costs are that script's, and the button's tooltip names the one
// a person feels.

import { execFile } from "node:child_process";

import type { Connection, FleetRestart } from "@armada/protocol";

/** `launchctl`'s answer to one call. */
export type Ran = { code: number; stdout: string; stderr: string };
export type Run = (args: string[]) => Promise<Ran>;

const launchctl: Run = (args) =>
  new Promise((resolve) => {
    execFile("launchctl", args, (error, stdout, stderr) => {
      const code = error === null ? 0 : typeof error.code === "number" ? error.code : 1;
      resolve({ code, stdout, stderr: stderr === "" && error !== null ? error.message : stderr });
    });
  });

/** The label `scripts/restart` loads Fleet under, and the one variable that moves it. */
function labelOf(env: NodeJS.ProcessEnv): string {
  return env["ARMADA_FLEET_LABEL"] ?? "com.armada.fleet";
}

export async function restartFleet(
  connection: Connection,
  run: Run = launchctl,
  env: NodeJS.ProcessEnv = process.env,
  uid: number = process.getuid?.() ?? 0,
): Promise<FleetRestart> {
  if (!("fleet" in connection)) {
    return { ok: false, why: "not_running", detail: "the runtime file names no Fleet" };
  }
  const service = `gui/${uid}/${labelOf(env)}`;
  const held = await run(["print", service]);
  // The pid launchd holds must be the one the runtime file names, as the
  // restart script requires: otherwise this is a Fleet started by hand, and
  // restarting the job would start a second one beside it.
  const pid = /^\s*pid = (\d+)$/m.exec(held.stdout)?.[1];
  if (held.code !== 0 || pid === undefined || Number(pid) !== connection.fleet.pid) {
    return {
      ok: false,
      why: "not_started_by_armada",
      detail: held.code !== 0 ? `${service} is not loaded` : `${service} holds pid ${pid ?? "none"}, the runtime file names ${connection.fleet.pid}`,
    };
  }
  const kicked = await run(["kickstart", "-k", service]);
  if (kicked.code !== 0) {
    return { ok: false, why: "refused", detail: kicked.stderr.trim() || `launchctl exited ${kicked.code}` };
  }
  return { ok: true };
}
