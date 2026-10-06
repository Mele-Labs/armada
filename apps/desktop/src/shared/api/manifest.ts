// The Manifest and Kit surface: the checkout's runs and drift, the file and its forms, Kit's servers and allowlists.
// A slice imports protocol and screens, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type {
  Outcome,
  RunOutputRead,
  CheckoutRunListRead,
  StartCheckoutRun,
  EditManifest,
  SaveManifestFile,
  CheckoutRunDiffRead,
  ManifestChecksRead,
  AddKitServer,
  ManifestReach,
  ReachesDrones,
  ManifestReading,
  CheckoutRunFollowed,
  CheckoutRunSheetRead,
  ManifestDriftRead,
} from "@armada/protocol";
import type {
  ManifestEditAnswer,
  ManifestFileRead,
  ManifestSaveAnswer,
  ManifestSpendRead,
} from "@armada/screens/src/editing";
import type { RepositoryAllowedCommandsRead } from "@armada/screens/src/manifest-allows";
import type {
  KitAllowedCommandsRead,
  KitInventoryRead,
  KitServersRead,
} from "@armada/screens/src/manifest-kit";

export type ManifestApi = {
  // The same rehearsal in the main checkout — Journey 9's *Running one*, the
  // Manifest surface. **Entries beside the seven above rather than a `jobId`
  // that may be `null` on each**: `/manifest/start_run` and
  // `/jobs/:id/start_run` are two operations, and a single capability taking
  // which would be a surface that reads as one act and performs two — the rule
  // the two kills are two entries for.

  /** Read what this repository's Manifest declares, or `false` to stop. Held
   * open by the Manifest surface, and by the palette, which lists off it. */
  watchCheckoutRunSheet: (want: boolean) => Promise<void>;
  /** One checkout run's output, or `null` to stop. */
  observeCheckoutRun: (runId: string | null) => Promise<void>;
  /**
   * Run one Check or Command in the main checkout, as it is on disk.
   *
   * **A name and nothing else.** There is no frozen Manifest to choose against
   * and no diff to narrow to, so neither of `StartRun`'s two flags has an
   * answer here. Opens `observeCheckoutRun` the moment the run exists.
   */
  startCheckoutRun: (body: StartCheckoutRun) => Promise<Outcome>;
  /** End a checkout run's process group. Its log keeps what printed. */
  stopCheckoutRun: (runId: string) => Promise<Outcome>;
  /**
   * Put back the files one checkout run changed, from the snapshot taken just
   * before it. **This tree holds a person's own uncommitted work**, which is
   * why the surface confirms by naming every path first.
   */
  undoCheckoutRun: (runId: string) => Promise<Outcome>;
  /** Every earlier checkout run, newest first, and what would not read. */
  listCheckoutRuns: () => Promise<CheckoutRunListRead>;
  /** One checkout run's log, read back as a window that says it is one. */
  getCheckoutRunOutput: (runId: string) => Promise<RunOutputRead>;
  /**
   * What one checkout run changed, **against the snapshot it took just before
   * it — never `HEAD`**. A snapshot that is gone is a reading that says so.
   * A read: it writes, stages and commits nothing.
   */
  getCheckoutRunDiff: (runId: string) => Promise<CheckoutRunDiffRead>;
  /**
   * Whether the repository still has what `armada.yml` names, or `false` to
   * stop. **A read, and free** — held open by the Manifest surface, and never
   * a dry-run.
   */
  watchManifestDrift: (want: boolean) => Promise<void>;
  /**
   * Run setup and every Check once in the main checkout, one after another.
   * **Only ever pressed.** Each step is followed as the checkout's own run.
   * `workspace` names a directory whose own `armada.yml` runs there; absent is the root's.
   */
  startCheckoutVerify: (workspace?: string) => Promise<Outcome>;
  /**
   * `armada.yml` as it is on disk, whole and unparsed — the Manifest surface's
   * file view. **No path crosses**: Fleet serves one repository and names the
   * file itself, so the renderer cannot aim this at anything else.
   */
  readManifestFile: () => Promise<ManifestFileRead>;
  /**
   * Put a corrected Manifest on disk, **only where it is still what the edit
   * started from** — `read` is that text, and Fleet refuses a save over a file
   * that moved, handing back what is there now. Writes and stops: no staging,
   * no commit. What Fleet made of it follows as `manifest.reread`.
   */
  saveManifestFile: (body: SaveManifestFile) => Promise<ManifestSaveAnswer>;
  /**
   * A form's edits, by key, **only where the file is still what the form was
   * drawn from**. Fleet splices each key, keeps every comment, and refuses a
   * result that would not load. Writes and stops.
   */
  editManifest: (body: EditManifest) => Promise<ManifestEditAnswer>;
  /** The costliest and the longest past Job here, for the budget warning. */
  readManifestSpend: () => Promise<ManifestSpendRead>;
  /** Every Check a Job's gate wrote or a Drone asked for, newest first. A read. */
  readManifestChecks: () => Promise<ManifestChecksRead>;
  /**
   * Every rule a person always-allowed for this repository, oldest first —
   * Fleet's own table since protocol 13.5. No path and no job id: it names no
   * Job and covers every job against this repository.
   */
  listRepositoryAllowedCommands: () => Promise<RepositoryAllowedCommandsRead>;
  /**
   * Take one back. Every job against this repository stops being granted it
   * from the next spawn on.
   */
  removeRepositoryAllowedCommand: (run: string) => Promise<RepositoryAllowedCommandsRead>;
  /**
   * Every MCP server in Kit, Kit's own default for each, this repository's
   * Manifest word over it, and whether a Drone dispatched here resolves it —
   * #1275. Every act below answers with the same whole list.
   */
  /**
   * The setup a person already works with, read from their agent harness's own
   * home — #1491. **A read and nothing more**: nothing it answers with reaches
   * a Drone, and nothing on this seam can make it.
   */
  readKitInventory: () => Promise<KitInventoryRead>;
  /**
   * Take one command out of Kit's allowlist, by the line as the inventory
   * spelled it, and answer with what the allowlist holds now. Machine-wide.
   * A 409 where no line is spelled that way.
   */
  removeKitAllowedCommand: (run: string) => Promise<KitAllowedCommandsRead>;
  listKitServers: () => Promise<KitServersRead>;
  /** Put one in Kit. **It reaches no Drone** until one of the two reaches below says so. */
  addKitServer: (adding: AddKitServer) => Promise<KitServersRead>;
  /** Take one out, and every Manifest's word about it with it. */
  forgetKitServer: (name: string) => Promise<KitServersRead>;
  /** Kit's own tier, for every Manifest that has not said otherwise. */
  setKitServerReach: (name: string, drones: ReachesDrones) => Promise<KitServersRead>;
  /** This Manifest's own word, or `null` to take it back and follow Kit again. */
  setManifestServerReach: (name: string, reach: ManifestReach | null) => Promise<KitServersRead>;
};

export type ManifestState = {
  /**
   * What Fleet's last read of `armada.yml` came to, or `null` because there
   * has not been one.
   *
   * **The second piece of state here that is not about a Job**, beside
   * `proposing` and for a related reason: a Manifest is Fleet's own, so there
   * is no row it belongs to.
   *
   * **This window's own**, like `repository` below — `PickedView`. Two windows
   * on two repositories each hold their own repository's reading.
   *
   * **Read on connect as well as folded from `manifest.reread`.** A refusal is
   * a standing condition, not an instant: the file and the values in force go
   * on disagreeing until somebody fixes the file, so a window opened a minute
   * after the save has to be able to find out. An event alone would tell only
   * whoever happened to be looking.
   */
  manifestReading: ManifestReading | null;
  /**
   * What this repository's Manifest declares, for the Manifest surface.
   *
   * **The second read here no Job scopes, and not for the reports' reason.**
   * It is the file Fleet is already holding, read before any Job exists —
   * which is the whole of why that surface can answer with the Board empty.
   *
   * Held open while the surface is showing **or the palette is up**: the
   * palette lists one row per Check and Command off this reading, and a read
   * scoped to the surface alone would leave those rows missing everywhere a
   * person would think to look for them. **This window's own** — `PickedView`.
   */
  checkoutRunSheet: CheckoutRunSheetRead;
  /** The checkout run a window is reading, as it prints. Its own socket,
   * `runFollowed`'s shape one owner over. One at a time, and this window's own. */
  checkoutRunFollowed: CheckoutRunFollowed;
  /**
   * Whether the repository still has what `armada.yml` names — drift, the
   * Manifest surface's free read on opening. **Held open by that surface
   * alone**, and read again when Fleet re-reads the file. This window's own.
   */
  manifestDrift: ManifestDriftRead;
};

export const MANIFEST_NOTHING_YET: ManifestState = {
  manifestReading: null,
  checkoutRunSheet: { state: "none" },
  checkoutRunFollowed: { state: "none" },
  manifestDrift: { state: "none" },
};

export const MANIFEST_CHANNELS = {
  // The same rehearsal in the main checkout — Journey 9's *Running one*.
  // Channels beside the Job's rather than a Job id that may be `null` on each:
  // a route under `/manifest` and a route under `/jobs/:id` are two
  // operations, and one capability taking which would read as one act and
  // perform two. The last is the checkout's own — a Job's diff is `readDiff`.
  watchCheckoutRunSheet: "bridge:watch-checkout-run-sheet",
  observeCheckoutRun: "bridge:observe-checkout-run",
  startCheckoutRun: "bridge:start-checkout-run",
  stopCheckoutRun: "bridge:stop-checkout-run",
  undoCheckoutRun: "bridge:undo-checkout-run",
  listCheckoutRuns: "bridge:list-checkout-runs",
  getCheckoutRunOutput: "bridge:get-checkout-run-output",
  getCheckoutRunDiff: "bridge:get-checkout-run-diff",
  // Journey 9's *Verify*, as two channels: drift is a read the surface holds
  // open, and Verify is an act behind its own button.
  watchManifestDrift: "bridge:watch-manifest-drift",
  startCheckoutVerify: "bridge:start-checkout-verify",
  // The Manifest file — Journey 9's *Editing*. Two entries, a read and a
  // write, and neither takes a path: Fleet names the file.
  readManifestFile: "bridge:read-manifest-file",
  saveManifestFile: "bridge:save-manifest-file",
  // The forms' two: edits as keys rather than text, and what past Jobs cost.
  editManifest: "bridge:edit-manifest",
  readManifestSpend: "bridge:read-manifest-spend",
  readManifestChecks: "bridge:read-manifest-checks",
  // A repository-wide always-allow, kept in Fleet's own table since 13.5 —
  // #836. Two entries, a read and a remove, on `readManifestFile`'s terms:
  // Fleet names the repository, so neither takes a path or an id.
  listRepositoryAllowedCommands: "bridge:list-repository-allowed-commands",
  removeRepositoryAllowedCommand: "bridge:remove-repository-allowed-command",
  // Kit's MCP servers — #1275. Five entries, on the same terms: Fleet names
  // the repository, so the Manifest tier needs no id from here.
  // The setup a person already has, read to be shown — #1491. Machine-wide
  // like Kit itself, and a read with nothing under it.
  readKitInventory: "bridge:read-kit-inventory",
  // Take a command out of Kit's allowlist, since 23.35. Machine-wide, as the read is.
  removeKitAllowedCommand: "bridge:remove-kit-allowed-command",
  listKitServers: "bridge:list-kit-servers",
  addKitServer: "bridge:add-kit-server",
  forgetKitServer: "bridge:forget-kit-server",
  setKitServerReach: "bridge:set-kit-server-reach",
  setManifestServerReach: "bridge:set-manifest-server-reach",
} as const;
