// The Manifest surface's checkout runs and drift, and the servers the Manifest declares.
//
// Beside `connection.ts` rather than inside it, for `command.ts`'s reason: a
// POST and a socket that only follows one run's output are neither the
// socket, the runtime file nor the state machine.
//
// **A rehearsal, never a verdict.** Nothing here writes Evidence or moves a
// Session: what changes is only ever the sheet's own reading.

import type { Outcome, RunOutputRead, ServerState } from "@armada/protocol";
import type { CheckoutRunListRead, StartCheckoutRun } from "@armada/protocol";
import type { CheckoutRunDiffRead } from "@armada/protocol";
import type { BridgeState, PickedView } from "../shared/bridge";
import { serversOf } from "./request";
import { CheckoutRunCommands, CheckoutRunSocket, CheckoutSheetReader, DriftReader } from "./checkout-runs";
import { ServerCommands } from "./servers";
import { Picked, type PickedByWindow } from "./picked";

/** The Manifest surface's own four, one set per window — `checkout-runs.ts`. */
type CheckoutFacades = {
  checkoutSheet: CheckoutSheetReader;
  checkoutFollow: CheckoutRunSocket;
  checkoutRuns: CheckoutRunCommands;
  drift: DriftReader;
};

/**
 * The Manifest surface's checkout runs and the servers, as one facade `connection.ts` holds one field
 * for. **Split out of `connection.ts`**, which named them as their own subject once the state machine
 * and the arrival handler had the file back at the 900-line gate.
 *
 * **The Manifest surface's four are a window's own** — `windows`, one set per window off that
 * window's own `Picked`.
 */
export class RehearsalConnection {
  private readonly publish: (change: Partial<BridgeState>) => void;
  private readonly publishToWindow: (windowId: number, change: Partial<PickedView>) => void;
  private readonly port: () => number | null;
  private readonly pickedByWindow: PickedByWindow;
  private readonly servers: ServerCommands;
  private readonly windows = new Map<number, CheckoutFacades>();

  constructor(wiring: {
    publish: (change: Partial<BridgeState>) => void;
    publishToWindow: (windowId: number, change: Partial<PickedView>) => void;
    port: () => number | null;
    pickedByWindow: PickedByWindow;
  }) {
    this.publish = wiring.publish;
    this.publishToWindow = wiring.publishToWindow;
    this.port = wiring.port;
    this.pickedByWindow = wiring.pickedByWindow;
    // Never held with a real listing: `startServer`'s own default, exercised only where a
    // caller supplies no `picked` — every real one does.
    this.servers = new ServerCommands({ port: this.port, picked: new Picked() });
  }

  /** This window's own Manifest surface facades, built on first ask. */
  private facadesFor(windowId: number): CheckoutFacades {
    let found = this.windows.get(windowId);
    if (found === undefined) {
      const picked = this.pickedByWindow.of(windowId);
      const checkoutFollow = new CheckoutRunSocket(
        (checkoutRunFollowed) => this.publishToWindow(windowId, { checkoutRunFollowed }),
        picked,
      );
      const checkoutSheet = new CheckoutSheetReader(
        (checkoutRunSheet) => this.publishToWindow(windowId, { checkoutRunSheet }),
        picked,
      );
      const checkoutRuns = new CheckoutRunCommands({
        port: this.port,
        picked,
        follow: (port, runId) => checkoutFollow.open(port, runId),
        refreshSheet: (port) => checkoutSheet.again(port),
      });
      const drift = new DriftReader((manifestDrift) => this.publishToWindow(windowId, { manifestDrift }), picked);
      found = { checkoutSheet, checkoutFollow, checkoutRuns, drift };
      this.windows.set(windowId, found);
    }
    return found;
  }

  /** The window closed. Its Manifest surface facades go with it. */
  dropWindow(windowId: number): void {
    const found = this.windows.get(windowId);
    if (found === undefined) return;
    found.checkoutSheet.close();
    found.checkoutFollow.close();
    found.drift.close();
    this.windows.delete(windowId);
  }

  close(): void {
    for (const windowId of this.windows.keys()) this.dropWindow(windowId);
  }

  /** Every server Fleet holds. Read once per connection; `server.*` on
   * `/events` keeps the list current from there — see `onServerEvent`. */
  async readServers(port: number): Promise<void> {
    const servers = await serversOf(port);
    if (servers !== null) this.publish({ servers });
  }

  /**
   * A run in the main checkout finished — `checkout_run.finished`, its own
   * kind because the record names no Job. Every window whose Manifest surface
   * holds its sheet open reads it again; the event arrives on all of them.
   */
  onCheckoutRunFinished(port: number): void {
    for (const { checkoutSheet } of this.windows.values()) {
      if (checkoutSheet.open) void checkoutSheet.again(port);
    }
  }

  /**
   * A server moved. **The sheet draws it off its own `servers[].instance`**, not the list above,
   * so the one holding it is read again — `onCheckoutRunFinished`'s terms. Fleet picks which
   * instance a sheet shows, live or last ended, and a second copy of that rule here would drift.
   */
  onServerMoved(port: number): void {
    for (const { checkoutSheet } of this.windows.values()) {
      if (checkoutSheet.open) void checkoutSheet.again(port);
    }
  }

  /** One `server.*` event, folded into the list it replaces a row in or joins. */
  onServerEvent(current: readonly ServerState[], row: ServerState): ServerState[] {
    return current.some((one) => one.id === row.id)
      ? current.map((one) => (one.id === row.id ? row : one))
      : [row, ...current];
  }

  /** Read what this window's own picked repository's Manifest declares, or `false` to stop. */
  async watchCheckoutRunSheet(windowId: number, want: boolean): Promise<void> {
    await this.facadesFor(windowId).checkoutSheet.want(this.port(), want);
  }

  /** One checkout run's output, or `null` to stop. */
  async observeCheckoutRun(windowId: number, runId: string | null): Promise<void> {
    this.facadesFor(windowId).checkoutFollow.open(this.port(), runId);
  }

  /** Run one Check or Command in this window's own checkout, and start following it. */
  startCheckoutRun(windowId: number, body: StartCheckoutRun): Promise<Outcome> {
    return this.facadesFor(windowId).checkoutRuns.startRun(body);
  }

  /** End a checkout run's process group. Its log keeps what printed. */
  stopCheckoutRun(windowId: number, id: string): Promise<Outcome> {
    return this.facadesFor(windowId).checkoutRuns.stopRun(id);
  }

  /** Put back the files one checkout run changed, from its own snapshot. */
  undoCheckoutRun(windowId: number, id: string): Promise<Outcome> {
    return this.facadesFor(windowId).checkoutRuns.undoRun(id);
  }

  /** Every earlier run in this window's own checkout, newest first. */
  listCheckoutRuns(windowId: number): Promise<CheckoutRunListRead> {
    return this.facadesFor(windowId).checkoutRuns.listRuns();
  }

  /** One checkout run's log, read back as a window that says it is one. */
  getCheckoutRunOutput(windowId: number, runId: string): Promise<RunOutputRead> {
    return this.facadesFor(windowId).checkoutRuns.getRunOutput(runId);
  }

  /** One checkout run's patch, against the snapshot it took. A read. */
  getCheckoutRunDiff(windowId: number, runId: string): Promise<CheckoutRunDiffRead> {
    return this.facadesFor(windowId).checkoutRuns.getRunDiff(runId);
  }

  /** Drift, or `false` to stop. Held open by this window's own Manifest surface alone. */
  async watchManifestDrift(windowId: number, want: boolean): Promise<void> {
    await this.facadesFor(windowId).drift.want(this.port(), want);
  }

  /**
   * This window's own picked repository moved, or gained its Manifest. What its Manifest
   * surface holds open is read again, and a run in the last one's checkout stops being followed.
   */
  async onRepositoryMoved(windowId: number, port: number): Promise<void> {
    const facades = this.facadesFor(windowId);
    facades.checkoutFollow.open(port, null);
    await Promise.all([
      facades.checkoutSheet.open ? facades.checkoutSheet.again(port) : null,
      facades.drift.open ? facades.drift.again(port) : null,
    ]);
  }

  /** Fleet re-read `armada.yml`; every window whose own pick reads that file draws its drift again. */
  onManifestReread(path: string, port: number): void {
    for (const [windowId, picked] of this.pickedByWindow.all()) {
      if (!picked.reads(path)) continue;
      const { drift } = this.facadesFor(windowId);
      if (drift.open) void drift.again(port);
    }
  }

  /** Verify — setup and every Check once, one after another, in this window's own checkout. */
  startCheckoutVerify(windowId: number, workspace?: string): Promise<Outcome> {
    return this.facadesFor(windowId).checkoutRuns.startVerify(workspace);
  }

  /** Start a declared server in the main checkout — `picked` is the calling window's own. */
  startServer(name: string, picked?: Picked): Promise<Outcome> {
    return this.servers.startServer(name, picked);
  }

  /** End a server's process group. Its log keeps what printed. */
  stopServer(id: string): Promise<Outcome> {
    return this.servers.stopServer(id);
  }
}
