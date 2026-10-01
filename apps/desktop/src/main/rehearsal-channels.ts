// Journey 9's run sheet, in a Job's worktree and in the window's own checkout — every channel here
// is `connection.rehearsal`'s, `rehearsal.ts`. Out of `index.ts` for the gate's length.

import type { IpcMain } from "electron";
import type { StartCheckoutRun, StartRun } from "@armada/protocol";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = {
  ipc: IpcMain;
  /** The open connection, or `null` before there is one. */
  connection: () => FleetConnection | null;
  /** The window an IPC call arrived from — `index.ts`'s own. */
  windowIdOf: (event: Electron.IpcMainInvokeEvent) => number;
};

export function handleRehearsal({ ipc, connection, windowIdOf }: Hosts): void {
  // The run sheet, Journey 9. Opened by the sheet rather than by the Job.
  // Every act below is `connection.rehearsal`'s — see `rehearsal.ts`.
  ipc.handle(CHANNELS.watchRunSheet, (_event, jobId: string | null) =>
    connection()?.rehearsal.watchRunSheet(jobId),
  );
  // One run's output, `followCheckOutput`'s reason and its own socket for it.
  ipc.handle(CHANNELS.observeRun, (_event, jobId: string | null, runId: string | null) =>
    connection()?.rehearsal.observeRun(jobId, runId),
  );
  // A rehearsal in this Job's own worktree — no Evidence, nothing on the Job
  // moves. Opens `observeRun` for the caller the moment the run exists.
  ipc.handle(CHANNELS.startRun, (_event, jobId: string, body: StartRun) =>
    connection()?.rehearsal.startRun(jobId, body),
  );
  ipc.handle(CHANNELS.stopRun, (_event, jobId: string, runId: string) =>
    connection()?.rehearsal.stopRun(jobId, runId),
  );
  ipc.handle(CHANNELS.undoRun, (_event, jobId: string, runId: string) =>
    connection()?.rehearsal.undoRun(jobId, runId),
  );
  ipc.handle(CHANNELS.listRuns, (_event, jobId: string) => connection()?.rehearsal.listRuns(jobId));
  ipc.handle(CHANNELS.getRunOutput, (_event, jobId: string, runId: string) =>
    connection()?.rehearsal.getRunOutput(jobId, runId),
  );
  // The same rehearsal in this window's own checkout — the Manifest surface. Held open
  // while that surface is showing or the palette is up, since the palette
  // lists one row per Check and Command off this reading.
  ipc.handle(CHANNELS.watchCheckoutRunSheet, (event, want: boolean) =>
    connection()?.rehearsal.watchCheckoutRunSheet(windowIdOf(event), want),
  );
  ipc.handle(CHANNELS.observeCheckoutRun, (event, runId: string | null) =>
    connection()?.rehearsal.observeCheckoutRun(windowIdOf(event), runId),
  );
  // A run in the tree a person is working in. **A name and nothing else** —
  // there is no frozen Manifest to choose against and no diff to narrow to.
  ipc.handle(CHANNELS.startCheckoutRun, (event, body: StartCheckoutRun) =>
    connection()?.rehearsal.startCheckoutRun(windowIdOf(event), body),
  );
  ipc.handle(CHANNELS.stopCheckoutRun, (event, runId: string) =>
    connection()?.rehearsal.stopCheckoutRun(windowIdOf(event), runId),
  );
  ipc.handle(CHANNELS.undoCheckoutRun, (event, runId: string) =>
    connection()?.rehearsal.undoCheckoutRun(windowIdOf(event), runId),
  );
  ipc.handle(CHANNELS.listCheckoutRuns, (event) => connection()?.rehearsal.listCheckoutRuns(windowIdOf(event)));
  ipc.handle(CHANNELS.getCheckoutRunOutput, (event, runId: string) =>
    connection()?.rehearsal.getCheckoutRunOutput(windowIdOf(event), runId),
  );
  // What one run changed, against the snapshot it took — never `HEAD`. A read.
  ipc.handle(CHANNELS.getCheckoutRunDiff, (event, runId: string) =>
    connection()?.rehearsal.getCheckoutRunDiff(windowIdOf(event), runId),
  );
  // Drift, held open by the Manifest surface; Verify, only ever pressed there.
  ipc.handle(CHANNELS.watchManifestDrift, (event, want: boolean) =>
    connection()?.rehearsal.watchManifestDrift(windowIdOf(event), want),
  );
  ipc.handle(CHANNELS.startCheckoutVerify, (event, workspace: unknown) =>
    connection()?.rehearsal.startCheckoutVerify(windowIdOf(event), typeof workspace === "string" ? workspace : undefined),
  );
}
