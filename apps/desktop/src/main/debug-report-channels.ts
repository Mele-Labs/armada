// The channels Copy debug report reads over: Fleet's text through the connection, and `armada report` run by main. The renderer
// names a repository and nothing else: no port, no program, no arguments.

import type { IpcMain } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";
import { armadaReport, locateArmada, rootOf, whereToRun, type Exec } from "./debug-report";

type Hosts = {
  ipc: IpcMain;
  connection: () => FleetConnection | null;
  /** The `armada` inside the packaged app, or `null` in a dev build. */
  bundled: string | null;
  /** A test hands in its own; the app uses launchd's program and a real spawn. */
  locate?: (bundled: string | null) => Promise<string | null>;
  run?: Exec;
};

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

export function handleDebugReport({ ipc, connection, bundled, locate = (held) => locateArmada(held), run }: Hosts): void {
  ipc.handle(CHANNELS.getDebugReport, (_event, root: unknown) => connection()?.debugReport.get(rootOf(root)) ?? unsent);
  ipc.handle(CHANNELS.runArmadaReport, async (_event, root: unknown) => {
    const where = rootOf(root);
    try {
      return { ok: true, text: await armadaReport(await locate(bundled), whereToRun(where), run) };
    } catch (failure) {
      return { ok: false, why: failure instanceof Error ? failure.message : String(failure) };
    }
  });
}
