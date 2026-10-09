// Fleet's health. Read by Overview's own tiles, and by the left column's Fleet panel
// (Bridge/1088), which is why `watchOverview` holds this open for the life of the window rather
// than only while Overview is showing.
//
// **Held while a surface wants them, `holding.ts`'s shape.** Health is a pull by Doctor's own terms.
// Read again when what they answer about may have moved: a listing or a pick (`repositories.ts`),
// a `manifest.reread` (`arrivals.ts`), and Refresh — which lists again, so it arrives as the first.
//
// **The last good reading stays up through a re-read.** Only opening draws the placeholders; a
// tile blanking on every save would read as a Fleet that stopped answering.

import type { FleetHealth } from "@armada/protocol";
import type { HealthRead } from "@armada/screens/src/overview-reads";
import type { PickedView } from "../shared/bridge";
import { ask } from "./request";
import type { Answer } from "./request";

export type OverviewWiring = {
  /** This window's own overlay — one `OverviewReads` per window, `connection.ts`'s `windowFacades`. */
  publish: (change: Partial<PickedView>) => void;
  port: () => number | null;
};

const NOT_CONNECTED = { ok: false, why: "not_connected" } as const;

export class OverviewReads {
  private readonly wiring: OverviewWiring;
  private open = false;
  /** Only the newest read publishes, so a pick that moved mid-read never draws the old scope. */
  private asked = 0;

  constructor(wiring: OverviewWiring) {
    this.wiring = wiring;
  }

  /** Read them, or drop what was read. */
  async watch(want: boolean): Promise<void> {
    this.open = want;
    if (!want) {
      this.wiring.publish({ health: { state: "none" } });
      return;
    }
    this.wiring.publish({ health: { state: "reading" } });
    await this.again(this.wiring.port());
  }

  /** Read again, where a surface has them open. Nothing open is no read. */
  async again(port: number | null): Promise<void> {
    if (!this.open) return;
    this.asked += 1;
    const asked = this.asked;
    if (port === null) {
      this.wiring.publish({ health: { state: "failed", outcome: NOT_CONNECTED } });
      return;
    }
    const health = await ask(port, "GET", "/health");
    // The surface closed, or a newer read went out, while this was in flight.
    if (!this.open || this.asked !== asked) return;
    this.wiring.publish({ health: healthOf(health) });
  }

  /** The read ends with the window. Nothing is published: the surface is gone. */
  close(): void {
    this.open = false;
  }
}

function healthOf(answer: Answer): HealthRead {
  return answer.ok === true
    ? { state: "read", health: answer.body as FleetHealth }
    : { state: "failed", outcome: answer.outcome };
}
