// Kit's MCP servers, read for the Manifest surface and changed there. #1275.
//
// `repository-allows.ts`'s shape: no run and no file, so `port()` and the pick
// are what a route under the picked Manifest needs. Every act answers with the
// whole list, so there is one read type and one fold.

import type {
  AddKitServer,
  KitAllowedCommands,
  KitInventory,
  ForgetKitServer,
  InstallKitPiece,
  KitPieceInstalled,
  KitPieceRemoved,
  KitServers,
  KitSourceFound,
  LookAtKitSource,
  ManifestReach,
  Outcome,
  ReachesDrones,
  RemoveKitAllowedCommand,
  RemoveKitPiece,
  RestoreKitPiece,
  SetKitServerReach,
  SetManifestServerReach,
} from "@armada/protocol";
import type {
  KitAllowedCommandsRead,
  KitInstallRead,
  KitInventoryRead,
  KitLookRead,
  KitRemoveRead,
  KitRestoreRead,
  KitServersRead,
} from "@armada/screens/src/manifest-kit";

import type { Picked } from "./picked";
import { ask, NO_WAIT, NOT_SET_UP } from "./request";

/** Fleet's own bound on a look, `LOOK_WITHIN`, and a margin, so Fleet gives up first and says so in words. */
const LOOK_MS = 30_000 + 5_000;

/** `get_kit_servers` and the four acts under it. */
export class KitCommands {
  private readonly port: () => number | null;
  private readonly picked: Picked;

  constructor(port: () => number | null, picked: Picked) {
    this.port = port;
    this.picked = picked;
  }

  /**
   * The setup a person already has, read to be shown — #1491. **Machine-wide**,
   * so no pick scopes it and the route takes no Manifest.
   */
  async inventory(): Promise<KitInventoryRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const answer = await ask(port, "GET", "/kit/inventory");
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, setup: answer.body as KitInventory };
  }

  /**
   * Take a command out of Kit's allowlist. **Machine-wide, like the read**, so
   * the route names no repository. `run` is the line as the inventory spelled
   * it; Fleet answers with what is left and refuses a line it does not hold.
   */
  async removeAllowedCommand(run: string): Promise<KitAllowedCommandsRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const body: RemoveKitAllowedCommand = { run };
    const answer = await ask(port, "POST", "/kit/allowed_commands/remove", body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, commands: answer.body as KitAllowedCommands };
  }

  /** `look_at_kit_source`: what a source is, plugin, skill or neither, before anything runs. Machine-wide. */
  async lookAtKitSource(body: LookAtKitSource): Promise<KitLookRead> {
    const answer = await this.machineWide("/kit/pieces/look", body, LOOK_MS);
    return answer.ok ? { ok: true, found: answer.body as KitSourceFound } : answer;
  }

  /**
   * `install_kit_piece`. **No wait of Bridge's own**: Fleet runs each command under its own bound, and
   * how many there are is the look's, so any figure here would cut an install Fleet is still running.
   */
  async installKitPiece(body: InstallKitPiece): Promise<KitInstallRead> {
    const answer = await this.machineWide("/kit/pieces/install", body, NO_WAIT);
    return answer.ok ? { ok: true, installed: answer.body as KitPieceInstalled } : answer;
  }

  /** `remove_kit_piece`: a skill to the Trash, or a plugin's uninstall run, which is why it waits as install does. */
  async removeKitPiece(body: RemoveKitPiece): Promise<KitRemoveRead> {
    const answer = await this.machineWide("/kit/pieces/remove", body, NO_WAIT);
    return answer.ok ? { ok: true, removed: answer.body as KitPieceRemoved } : answer;
  }

  /** `restore_kit_piece`: a skill's folder back out of the Trash, by the place remove answered with. */
  async restoreKitPiece(body: RestoreKitPiece): Promise<KitRestoreRead> {
    const answer = await this.machineWide("/kit/pieces/restore", body);
    return answer.ok ? { ok: true, setup: answer.body as KitInventory } : answer;
  }

  /** Every server in Kit, with what a Drone dispatched here resolves. */
  async list(): Promise<KitServersRead> {
    return this.read("GET", "/kit/servers");
  }

  /** Put one in Kit. It reaches no Drone until a person says so. */
  async add(adding: AddKitServer): Promise<KitServersRead> {
    return this.read("POST", "/kit/servers/add", adding);
  }

  /** Take one out, and every Manifest's word about it with it. */
  async forget(name: string): Promise<KitServersRead> {
    const body: ForgetKitServer = { name };
    return this.read("POST", "/kit/servers/forget", body);
  }

  /** Kit's own tier, across every Manifest that has not said otherwise. */
  async setKitReach(name: string, drones: ReachesDrones): Promise<KitServersRead> {
    const body: SetKitServerReach = { name, drones };
    return this.read("POST", "/kit/servers/reach", body);
  }

  /**
   * This Manifest's own word, or `null` to take it back.
   *
   * **`null` is sent rather than left out**, which is what Fleet refuses a body
   * without: a dropped key would throw a person's word away and answer 200.
   */
  async setManifestReach(name: string, reach: ManifestReach | null): Promise<KitServersRead> {
    const body: SetManifestServerReach = { name, reach };
    return this.read("POST", "/kit/servers/manifest_reach", body);
  }

  private async machineWide(
    route: string,
    body: unknown,
    waitMs?: number,
  ): Promise<{ ok: true; body: unknown } | { ok: false; outcome: Outcome }> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const answer = await ask(port, "POST", route, body, waitMs);
    return answer.ok === true ? { ok: true, body: answer.body } : { ok: false, outcome: answer.outcome };
  }

  private async read(method: "GET" | "POST", route: string, body?: unknown): Promise<KitServersRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const path = this.picked.manifest(route);
    if (path === null) return { ok: false, outcome: NOT_SET_UP };
    const answer = await ask(port, method, path, body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, kit: answer.body as KitServers };
  }
}
