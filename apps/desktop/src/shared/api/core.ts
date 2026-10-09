// What every surface needs: the connection, the Jobs list, the rail's pick, servers, and the few acts no surface owns.
// A slice imports protocol and screens, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type {
  BuildSource,
  Followed,
  FleetBuildReport,
  FleetRestart,
  Outcome,
  BridgeIdentity,
  Connection,
  Holdings,
  FleetCapacity,
  JobSummary,
  UnreadableJob,
  ServerList,
} from "@armada/protocol";
import type { Pattern } from "../haptics";

/** `S` is the whole app's state, which `state` and `subscribe` hand over; Core cannot name it. */
export type CoreApi<S = CoreState> = {
  state: () => Promise<S>;
  subscribe: (onState: (state: S) => void) => () => void;
  /** Whether a walk window has focus, as it changes — Bridge dims behind it. */
  onWalkFocus: (onFocus: (focused: boolean) => void) => () => void;
  /**
   * Pick the repository every per-repository read and act names, by a root `holds.repositories`
   * lists, or `null` for All repositories, which names none. What changed arrives as state:
   * `repository`, and the reads held open, taken again.
   */
  pickRepository: (root: string | null) => Promise<void>;
  /** Start a declared server — this Job's worktree, or the main checkout with
   * no Job. `server.serving`/`server.exited` follow as events. */
  startServer: (name: string, jobId?: string) => Promise<Outcome>;
  /** End a server's process group. Its log keeps what printed. */
  stopServer: (serverId: string) => Promise<Outcome>;
  /** Open one server link in the system browser. **No surface navigates.** A
   * server id and the link's own address — main checks it against the links
   * it holds for that server before handing anything to the OS. */
  openServerLink: (serverId: string, url: string) => Promise<Followed>;
  /** Open a link written in a model's text, in the system browser. **The one
   * entry that sends an address**: such a link has no id main could look it
   * up by. Main opens `http(s):` and refuses everything else by name. */
  openLink: (address: string) => Promise<Followed>;
  /**
   * Restart the job that runs Fleet, onto the `armada` that is installed. **The
   * one capability that acts on Fleet's own process** and not through its
   * wire: it is `launchctl` in main, which is why a Fleet too old for any route
   * can still be restarted. `restart-fleet.ts` says what it promises.
   */
  restartFleet: () => Promise<FleetRestart>;
  /**
   * Restart Fleet and Bridge onto latest `main`, or onto a refreshed preview.
   * **Fleet starts `scripts/restart-build` detached and answers at once**: what
   * it came to arrives as `fleetBuild`, `restarting` while it works and `failed`
   * with one line if it did not take. `adopt` is a person's say-so to restart
   * with a Drone working.
   */
  changeFleetBuild: (build: BuildSource, adopt: boolean) => Promise<Outcome>;
  /**
   * Where a pressed notification says to go.
   *
   * **The one entry here the renderer cannot initiate.** Every other capability
   * is the window asking Fleet for something; this is main handing over a press
   * that happened outside the window — possibly while there was no window — and
   * the renderer's only part is to go there.
   *
   * It grants nothing: what arrives is a job id this window already draws, or
   * `null`. Subscribing twice is two callbacks, and the returned function is
   * how one stops.
   */
  onSummoned: (onGo: (to: Summons) => void) => () => void;
  /**
   * A trackpad swipe or a browser back/forward key, which only main hears.
   * One-way and `"back" | "forward"` only; it grants nothing.
   */
  onHistory: (onStep: (step: HistoryStep) => void) => () => void;
  /**
   * Play a trackpad pattern for a press Fleet has just answered. Returns
   * nothing and waits for nothing; main plays one of the two patterns and
   * ignores anything else. Touch, in `docs/contracts/design-system.md`.
   */
  tap: (pattern: Pattern) => void;
};

export type CoreState = {
  connection: Connection;
  /** Where a Bridge failure points. Never a Job's identity. */
  bridge: BridgeIdentity;
  jobs: JobSummary[];
  /** Rows the store refused. Shown, never merged into `jobs` as a placeholder. */
  unreadable: UnreadableJob[];
  /**
   * How full the fleet is, and what holds the next Drone back.
   *
   * **`null` until Fleet has answered once**, which is a Bridge that has not
   * asked rather than a Fleet with room — the two are drawn differently and a
   * zeroed placeholder would make them one. Re-read whenever a Job moves,
   * because a Job moving is the only thing that changes the occupancy, and the
   * machine reading rides along on the same call.
   */
  capacity: FleetCapacity | null;
  /**
   * The build Fleet runs on and where it stands against `origin/main`.
   * **`null` where Fleet serves no build to choose** or has not answered, which
   * draws no build section; a Fleet that is restarting keeps the last reading.
   * Read on every connection and on a minute's interval, since a fetch moves it
   * and no event says so.
   */
  fleetBuild: FleetBuildReport | null;
  /** Events Fleet dropped before Bridge saw them, since the window opened. */
  missed: number;
  /** When the Jobs above were last current, in epoch milliseconds. */
  readAt: number | null;
  /**
   * What Fleet holds, and therefore what a proposal may name.
   *
   * Read over the one connection like everything else here. The composer used
   * to offer a text field for a pasted id, because nothing served these — and a
   * pasted id was accepted by Fleet unchecked. Both halves of that are fixed:
   * Fleet refuses an id it does not hold, and this is what the form offers so
   * nobody has to guess at one.
   *
   * **`leftOut` is this window's own**, off `PickedView` — every other field here is shared.
   */
  holds: Holdings;
  /**
   * The root of the repository this window's own rail picked, which every per-repository read
   * and act this window sends names. `null` is no one repository: All repositories, where a
   * window opens, or a Fleet that lists none.
   *
   * **This window's own** — `PickedView`, `main/picked.ts`'s `PickedByWindow`. Two windows may
   * each have a different one; picking in one never moves the other's.
   */
  repository: string | null;
  /**
   * Every server Fleet holds, each Job's and the main checkout's.
   *
   * **Read once per connection, kept current by folding `server.*` off
   * `/events`**, `capacity`'s terms. Not scoped to the open Job — *Where
   * things are* keeps a *Serving* row up after the sheet that started it
   * closes.
   */
  servers: ServerList;
};

export const CORE_NOTHING_YET: CoreState = {
  connection: { state: "reading" },
  // Main resolves the log path from the home it can see. Until it answers, the
  // renderer does not know it and does not name one.
  bridge: { auditPath: null, fleetProtocol: null },
  jobs: [],
  unreadable: [],
  capacity: null,
  fleetBuild: null,
  missed: 0,
  readAt: null,
  holds: { workflows: [], manifests: [], models: null, repositories: [] },
  repository: null,
  servers: { servers: [] },
};

export const CORE_CHANNELS = {
  state: "bridge:state",
  changed: "bridge:changed",
  /** A walk window took or gave up focus, so Bridge dims behind it or lifts the dim. */
  walkFocused: "bridge:walk-focused",
  // The rail's pick. A root and nothing else; main ignores one Fleet does not list.
  pickRepository: "bridge:pick-repository",
  startServer: "bridge:start-server",
  stopServer: "bridge:stop-server",
  openServerLink: "bridge:open-server-link",
  openLink: "bridge:open-link",
  restartFleet: "bridge:restart-fleet",
  changeFleetBuild: "bridge:change-fleet-build",
  summoned: "bridge:summoned",
  // A swipe or a browser key, from the OS to the window. Sent by main, never invoked.
  history: "bridge:history",
  // A press Fleet answered, felt on the trackpad. Sent, never invoked: nothing waits on it.
  tap: "bridge:tap",
} as const;

/**
 * Where a pressed notification lands.
 *
 * **A job, or the set.** A telling about one job opens that job. A telling
 * about several cannot open one of them without choosing for somebody, so it
 * lands on the Needs-you tab — which is the set the notification was derived
 * from, and the same one keyboard `2` reaches.
 *
 * It travels main → renderer only. Nothing the renderer sends can produce one.
 */
export type Summons = { jobId: string | null };

/** Which way a gesture the OS recognised asks to move through visited places. */
export type HistoryStep = "back" | "forward";
