// The build Fleet runs on, as the Fleet panel draws it: read on every connection and on a minute's
// interval, and the restart onto another build.
//
// **No event says the build moved**, so it is read. A background fetch in Fleet moves the position
// without a Job moving, and a restart moves the build with the socket down.
//
// **A restart is followed, not awaited.** `POST /fleet/build/change` answers 202 as the detached
// script starts. What it came to is the next read: the report says `restarting` while the script
// works and `failed` with one line if it did not take. A script that fails before it stops Fleet
// leaves Fleet up with no event, which is why the follow reads every two seconds. One that does
// stop Fleet is read again by the resync on the Fleet that comes back.

import type { BuildSource, FleetBuildReport, Outcome } from "@armada/protocol";
import type { BridgeState } from "../shared/bridge";
import { ask as asked } from "./request";
import type { Answer } from "./request";

/** How often the build is read while nothing restarts. */
export const WATCH_MS = 60_000;
/** How often it is read while one does. */
export const FOLLOW_MS = 2_000;
/** A restart still `restarting` after this is a script that died, and the follow stops. */
const FOLLOW_FOR_MS = 20 * 60_000;

export type FleetBuildWiring = {
  /** The port Fleet answers on now, or `null` while it is not connected. */
  port: () => number | null;
  current: () => BridgeState;
  publish: (change: Partial<BridgeState>) => void;
  /** `ask`, or a test's. */
  ask?: (port: number, method: "GET" | "POST", path: string, body?: unknown) => Promise<Answer>;
  now?: () => number;
};

export class FleetBuilds {
  private readonly wiring: FleetBuildWiring;
  private watching: ReturnType<typeof setInterval> | null = null;
  private following: ReturnType<typeof setTimeout> | null = null;
  private followUntil = 0;

  constructor(wiring: FleetBuildWiring) {
    this.wiring = wiring;
  }

  private ask(port: number, method: "GET" | "POST", path: string, body?: unknown): Promise<Answer> {
    return (this.wiring.ask ?? asked)(port, method, path, body);
  }

  /**
   * Read the build once.
   *
   * **A refusal publishes `null`**, which draws no section: this Fleet serves no build to choose.
   * **A Fleet that did not answer publishes nothing**, because the commonest cause is a restart
   * under way, and the section going blank in the middle of one would say it had finished.
   */
  async read(port: number): Promise<void> {
    const answer = await this.ask(port, "GET", "/fleet/build");
    if (answer.ok === true) {
      this.wiring.publish({ fleetBuild: answer.body as FleetBuildReport });
    } else if (answer.outcome.ok === false && answer.outcome.why === "refused") {
      this.wiring.publish({ fleetBuild: null });
    }
  }

  /** Read it now and again every minute, for as long as this connection stands. */
  watch(port: number): void {
    void this.read(port);
    if (this.watching !== null) clearInterval(this.watching);
    this.watching = setInterval(() => {
      const now = this.wiring.port();
      if (now !== null) void this.read(now);
    }, WATCH_MS);
  }

  /**
   * Ask Fleet to restart onto `build`, with `adopt` as the person's say-so to do it past a working
   * Drone. **The report is marked `restarting` at once**, so the panel shows the work before the
   * first read comes back; a refusal leaves it as it was.
   */
  async change(build: BuildSource, adopt: boolean): Promise<Outcome> {
    const port = this.wiring.port();
    if (port === null) return { ok: false, why: "not_connected" };
    const answer = await this.ask(port, "POST", "/fleet/build/change", { build, adopt });
    const held = this.wiring.current().fleetBuild;
    if (answer.ok !== true) {
      // Said where the panel already says why a restart did not take.
      const said = answer.outcome;
      const why = said.ok === false && said.why === "refused" ? said.error.message : said.ok === false && said.why === "transport" ? said.detail : "Fleet did not answer";
      if (held !== null) this.wiring.publish({ fleetBuild: { ...held, failed: why } });
      return said;
    }
    if (held !== null) {
      const { failed: _gone, ...rest } = held;
      this.wiring.publish({ fleetBuild: { ...rest, restarting: build } });
    }
    this.follow();
    return { ok: true };
  }

  private follow(): void {
    const now = this.wiring.now ?? Date.now;
    this.followUntil = now() + FOLLOW_FOR_MS;
    if (this.following !== null) clearTimeout(this.following);
    const tick = () => {
      this.following = null;
      const port = this.wiring.port();
      void (port === null ? Promise.resolve() : this.read(port)).then(() => {
        const held = this.wiring.current().fleetBuild;
        if (held?.restarting === undefined && port !== null) return;
        if (now() >= this.followUntil) return;
        this.following = setTimeout(tick, FOLLOW_MS);
      });
    };
    this.following = setTimeout(tick, FOLLOW_MS);
  }

  close(): void {
    if (this.watching !== null) clearInterval(this.watching);
    if (this.following !== null) clearTimeout(this.following);
    this.watching = null;
    this.following = null;
  }
}
