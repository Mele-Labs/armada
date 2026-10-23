// The Mods list's words, Fleet, tweak and tracker rows, over Fleet: its mod list, this machine's switch for each and Promote. A
// `TaughtSource` like the mock's was, so `ModList` and the System page read it unchanged. `docs/concepts/mods.md`.
//
// **Fleet's list says a mod's kind, what it does in a line and what it was taught on.** What a row opens to is the file Fleet checked
// for it (`validate_mod`), read once for each change of the mod. A layout mod is listed from the layout source, so only what it was
// taught on and what it holds are kept here, for its row to say.

import type { ModChecked, ModSummary, Outcome } from "@armada/protocol";
import { trackerOf, type TaughtDetail, type TaughtKind, type TaughtMod, type TaughtPart, type TaughtSource, type TaughtState } from "@armada/settings";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { BridgeState } from "../../shared/bridge";
import { modOpened } from "./open-mod";
import { askToTell } from "./tell";
import { fleetAlso, partsOf } from "./taught-parts";

type Facts = Pick<BridgeState, "mods">;

/** What the list is read through: `window.armada` in the app, a fake in a test. */
export type FleetTaught = {
  state: () => Promise<Facts>;
  subscribe: (on: (facts: Facts) => void) => () => void;
  setModEnabled: (name: string, enabled: boolean) => Promise<Outcome>;
  promoteMod: (name: string) => Promise<Outcome>;
  validateMod: (name: string) => Promise<ModChecked | null>;
};

const KINDS: readonly TaughtKind[] = ["words", "fleet", "tweak", "tracker"];
const isTaught = (kind: ModSummary["kind"]): kind is TaughtKind => (KINDS as readonly (string | undefined)[]).includes(kind);

/** A window that lists no taught mod: safe mode, which reads no mod. */
const NOTHING: TaughtState = { mods: [], details: {} };
export const NO_TAUGHT: TaughtSource = {
  get: () => NOTHING,
  subscribe: () => () => undefined,
  setEnabled: () => undefined,
  promote: () => undefined,
  open: () => undefined,
};

export function createFleetTaught(fleet: FleetTaught, tell: (sentence: string) => void = askToTell): TaughtSource {
  let rows: readonly ModSummary[] = [];
  let seen = "";
  // A switch just pressed, drawn at once; Fleet's next list replaces it.
  const switched = new Map<string, boolean>();
  const branches = new Map<string, string>();
  // What each mod holds, from the file Fleet last checked for it, and the change of the mod that read was of.
  const checked = new Map<string, { at: string; parts: readonly TaughtPart[]; line?: string }>();
  const asking = new Set<string>();
  const listeners = new Set<() => void>();

  const build = (): TaughtState => {
    const mods = rows.flatMap((row): TaughtMod[] =>
      !isTaught(row.kind)
        ? []
        : [
            {
              name: row.name,
              kind: row.kind,
              description: row.valid ? (checked.get(row.name)?.line ?? row.description ?? "") : (row.reason ?? "Did not pass its checks"),
              on: row.taught_on ?? "",
              enabled: switched.get(row.name) ?? row.enabled,
              ...(branches.has(row.name) ? { branch: branches.get(row.name)! } : {}),
              parts: checked.get(row.name)?.parts ?? [],
              ...(row.kind === "tracker" ? { tracker: trackerOf(row) } : {}),
            },
          ],
    );
    const details: Record<string, TaughtDetail> = {};
    for (const row of rows) {
      if (row.kind === "layout" && row.taught_on !== undefined) details[row.name] = { on: row.taught_on, parts: checked.get(row.name)?.parts ?? [] };
    }
    const named = modOpened.get();
    return { mods, details, ...(named !== undefined && (details[named] !== undefined || mods.some((one) => one.name === named)) ? { opened: named } : {}) };
  };

  let held = build();
  const refresh = () => {
    held = build();
    listeners.forEach((on) => on());
  };

  const ask = (row: ModSummary) => {
    const at = row.changed_at ?? "";
    const key = `${row.name}@${at}`;
    if (checked.get(row.name)?.at === at || asking.has(key)) return;
    asking.add(key);
    fleet.validateMod(row.name).then(
      (read) => {
        asking.delete(key);
        if (read === null) return;
        // A Fleet mod's line is its rules in the words its row opened and the Logbook use, not the file's own description.
        const line = read.fleet === undefined ? undefined : fleetAlso(read.fleet);
        checked.set(row.name, { at, parts: partsOf(read), ...(line === undefined ? {} : { line: `${line.charAt(0).toUpperCase()}${line.slice(1)}` }) });
        refresh();
      },
      () => void asking.delete(key),
    );
  };

  const ingest = (next: Facts) => {
    const key = JSON.stringify(next.mods);
    if (key === seen) return;
    seen = key;
    rows = next.mods?.mods ?? [];
    switched.clear();
    for (const name of [...checked.keys()]) if (!rows.some((row) => row.name === name)) checked.delete(name);
    refresh();
    // A Words, Fleet or tweak mod holds a file worth reading; a layout only where it was taught, since the layout source lists the rest.
    rows.filter((row) => row.valid && (row.kind === "layout" ? row.taught_on !== undefined : row.kind === "words" || row.kind === "fleet" || row.kind === "tweak")).forEach(ask);
  };
  fleet.state().then(ingest, () => undefined);
  fleet.subscribe(ingest);
  modOpened.subscribe(refresh);

  return {
    get: () => held,
    subscribe: (on) => {
      listeners.add(on);
      return () => void listeners.delete(on);
    },
    setEnabled: (name, enabled) => {
      switched.set(name, enabled);
      refresh();
      void fleet.setModEnabled(name, enabled).then((outcome) => {
        if (outcome.ok) return;
        switched.delete(name);
        refresh();
        tell(refusalWords(outcome));
      });
    },
    promote: (name) => {
      void fleet.promoteMod(name).then((outcome) => {
        if (!outcome.ok) return tell(refusalWords(outcome));
        if (outcome.modPromoted === undefined) return;
        branches.set(name, outcome.modPromoted.branch);
        refresh();
      });
    },
    open: (name) => modOpened.set(name),
  };
}
