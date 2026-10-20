// The flight controls Kit holds and each repository's, kept in memory for the life of the window, and a shell line that ends the way
// the fake runner's does: `vitest` fails, every other line exits clean. Every repository's are kept On this Mac. A scenario that wants a
// file that will not read, another place, or a line that times out, supplies its own through `Scenario.behaves`.
//
// **A repository's committed tiles are the scenario's** (`draft.committedTiles`), and a shell tile among them asks before it runs, as
// Fleet asks: a press naming it is refused until its command's hash is approved, and runs the committed command, not the one sent.

import type { FlightControls, RepositoryFlightControls, TileAsk, WireError } from "@armada/protocol";

import type { FlightControlsApi, SessionShellAnswer } from "../../../../shared/api/flight-controls";
import type { Slice } from "../fake-context";
import { RUN_DEFAULT, RUN_OUTPUT } from "../sessions/script";

const EMPTY: FlightControls = { version: 1, job: [], deck: [] };

/** A stand in for Fleet's hash: the same command always gives the same one, and another command another. */
export function commandHash(command: string): string {
  let hash = 5381;
  for (const char of command) hash = ((hash * 33) ^ char.charCodeAt(0)) >>> 0;
  return `mock-${hash.toString(16)}`;
}

const refusal = (code: string, message: string, fields: Record<string, unknown> = {}) => {
  const error: WireError = { code, message, run_id: "run-mock", fields, chain: [] };
  return { ok: false as const, outcome: { ok: false as const, why: "refused" as const, error } };
};

export const flightControls: Slice<FlightControlsApi, Record<string, never>> = {
  name: "flightControls",
  state: {},
  api: (scenario) => {
    let held: FlightControls = structuredClone(EMPTY);
    const repositories = new Map<string, FlightControls>();
    const committed = scenario.draft?.committedTiles ?? {};
    const approved = new Map<string, Map<string, string>>();
    const committedOf = (repository: string) => [...(committed[repository]?.job ?? []), ...(committed[repository]?.deck ?? [])];
    const shellOf = (repository: string, id: string) => committedOf(repository).find((one) => one.id === id && one.kind === "shell");
    const askingOf = (repository: string): TileAsk[] =>
      committedOf(repository)
        .filter((one) => one.kind === "shell")
        .map((one) => {
          const command = one.text ?? "";
          const command_hash = commandHash(command);
          return { id: one.id, command, command_hash, approved: approved.get(repository)?.get(one.id) === command_hash };
        });
    const answer = (repository: string): { ok: true; value: RepositoryFlightControls } => {
      const asking = askingOf(repository);
      return {
        ok: true,
        value: {
          repository,
          place: "this_mac",
          read: { what: "read", controls: structuredClone(repositories.get(repository) ?? EMPTY) },
          in_repository: { what: "read", controls: structuredClone(committed[repository] ?? EMPTY) },
          ...(asking.length === 0 ? {} : { asking }),
        },
      };
    };
    const ran = (command: string): SessionShellAnswer => {
      const lines = RUN_OUTPUT.find(([match]) => command.includes(match))?.[1] ?? RUN_DEFAULT;
      return { ok: true, value: { exit: command.includes("vitest") ? 1 : 0, timed_out: false, output: `${lines.join("\n")}\n`, truncated: false } };
    };
    return {
      getFlightControls: async () => ({ ok: true, value: { what: "read", controls: structuredClone(held) } }),
      saveFlightControls: async (controls) => {
        held = structuredClone(controls);
        return { ok: true, value: { what: "read", controls: structuredClone(held) } };
      },
      getRepositoryFlightControls: async (repository) => answer(repository),
      saveRepositoryFlightControls: async (repository, controls) => {
        repositories.set(repository, structuredClone(controls));
        return answer(repository);
      },
      approveRepositoryTile: async (body) => {
        const tile = shellOf(body.repository, body.tile_id);
        if (tile === undefined) return refusal("fleet.tile_not_in_repository", `The repository holds no shell tile ${body.tile_id}`);
        if (commandHash(tile.text ?? "") !== body.command_hash) return refusal("fleet.tile_command_changed", "The tile's command changed since it was shown");
        approved.set(body.repository, (approved.get(body.repository) ?? new Map<string, string>()).set(body.tile_id, body.command_hash));
        return answer(body.repository);
      },
      runSessionShell: async (_sessionId, command, tile) => {
        if (tile === undefined) return ran(command);
        const one = shellOf(tile.repository, tile.id);
        if (one === undefined) return refusal("fleet.tile_not_in_repository", `The repository holds no shell tile ${tile.id}`);
        const line = one.text ?? "";
        const hash = commandHash(line);
        if (approved.get(tile.repository)?.get(tile.id) !== hash) {
          return refusal("fleet.tile_not_approved", "This Mac has not approved this tile's command", { repository: tile.repository, tile_id: tile.id, command: line, command_hash: hash });
        }
        return ran(line);
      },
    };
  },
};
