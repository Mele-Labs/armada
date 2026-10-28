// What every surface needs: the connection, the rail's pick, servers, and the few acts no surface owns.
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
  ServerList,
} from "@armada/protocol";
import type { PanelGo } from "@armada/studios/api";
import type { Pattern } from "../haptics";
import type { PullLink } from "../pull-link";
import type { StateChange } from "../state-change";

/** `S` is the whole app's state, which `state` and `subscribe` hand over; Core cannot name it. */
export type CoreApi<S = CoreState> = {
  state: () => Promise<S>;
  /**
   * What moved in the state, as main tells it: the whole of it once and then only what differs, `shared/state-change.ts`.
   * **Surfaces do not read this**; `renderer/src/state-feed.ts` folds it into `subscribe`'s whole states.
   */
  subscribeChanges: (onChange: (change: StateChange<S>) => void) => () => void;
  /** The whole state, as the first change a window folds the rest onto, and again where one reaches it out of order. */
  syncState: () => Promise<StateChange<S>>;
  /** Whether a walk window has focus, as it changes — Bridge dims behind it. */
  onWalkFocus: (onFocus: (focused: boolean) => void) => () => void;
  /**
   * Pick the repository every per-repository read and act names, by a root `holds.repositories`
   * lists, or `null` for All repositories, which names none. What changed arrives as state:
   * `repository`, and the reads held open, taken again.
   */
  pickRepository: (root: string | null) => Promise<void>;
  /** Start a declared server in the main checkout. `server.serving`/`server.exited` follow as events. */
  startServer: (name: string) => Promise<Outcome>;
  /** End a server's process group. Its log keeps what printed. */
  stopServer: (serverId: string) => Promise<Outcome>;
  /** Open one server link in the system browser. **No surface navigates.** A
   * server id and the link's own address — main checks it against the links
   * it holds for that server before handing anything to the OS. */
  openServerLink: (serverId: string, url: string) => Promise<Followed>;
  /** Open a link written in a model's text, or an MCP server's sign-in page
   * (`SessionMcpActed.sign_in.url`), in the system browser. **The one entry
   * that sends an address**: such a link has no id main could look it up by.
   * Main opens `http(s):` and refuses everything else by name. */
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
   * It grants nothing: what arrives is a Session id this window already draws. Subscribing twice is two callbacks, and the returned function is
   * how one stops.
   */
  onSummoned: (onGo: (to: Summons) => void) => () => void;
  /**
   * A link to a pull request opened from outside, `armada://pull/<owner>/<name>/<n>`: the repository and number it names. Main only sends one while
   * `features.deepLinks` is on, and holds it for a window that is not up yet. It grants nothing: the window finds the repository among those it draws.
   */
  onPullLink: (onLink: (link: PullLink) => void) => () => void;
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
  /** How many calls the Cockpit holds for him, for the dock's badge; none clears it. Sent, waits for nothing. */
  dockCount: (calls: number) => void;
};

export type CoreState = {
  connection: Connection;
  /** Where a Bridge failure points. */
  bridge: BridgeIdentity;
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
  /** Whether Bridge opened on the picture it kept on disk (`main/remembered.ts`) rather than on nothing. Set at launch and never cleared: it is how the Cockpit knows not to power on. */
  drawnFromDisk: boolean;
  /**
   * When Bridge last had Fleet's picture whole, in epoch milliseconds: the last resync. `null`
   * until Fleet has answered once. It says how stale an unreachable Fleet's picture is.
   */
  readAt: number | null;
  /**
   * What Fleet holds, and therefore what a dispatch may name.
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
   * Every server Fleet holds.
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
  fleetBuild: null,
  missed: 0,
  drawnFromDisk: false,
  readAt: null,
  holds: { workflows: [], manifests: [], models: null, repositories: [] },
  repository: null,
  servers: { servers: [] },
};

export const CORE_CHANNELS = {
  state: "bridge:state",
  /** Main to a window: a `StateChange`. */
  changed: "bridge:changed",
  syncState: "bridge:sync-state",
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
  // A pull request's link opened from outside, main to the window. Sent by main, never invoked.
  pullLinked: "bridge:pull-linked",
  // A swipe or a browser key, from the OS to the window. Sent by main, never invoked.
  history: "bridge:history",
  // A press Fleet answered, felt on the trackpad. Sent, never invoked: nothing waits on it.
  tap: "bridge:tap",
  // The Cockpit's calls, counted for the dock. Sent, never invoked.
  dockCount: "bridge:dock-count",
} as const;

/**
 * Where a pressed notification, or a walk window's panel, lands: the Session it names, and for the panel
 * what it asked to open of that Session besides. It travels main → renderer only.
 */
export type Summons = { sessionId: string; then?: Exclude<PanelGo, { kind: "session" }> };

/** Which way a gesture the OS recognised asks to move through visited places. */
export type HistoryStep = "back" | "forward";
