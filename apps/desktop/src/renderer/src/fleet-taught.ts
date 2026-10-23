// The Mods list's words, Fleet, tweak and tracker rows, over Fleet: its mod list, this machine's switch for each and Promote. A
// `TaughtSource` like the mock's was, so `ModList` and the System page read it unchanged. `docs/concepts/mods.md`.
//
// **Fleet's list says a mod's kind, what it does in a line and what it was taught on**, and a row is drawn from no more than that.
// A mock Fleet that words more (`teachFlavour().mod`) gives a row what it holds, and the row opens to read it. A layout mod
// is listed from the layout source, so only what it was taught on is kept here, for its row to say.

import type { ModSummary, Outcome } from "@armada/protocol";
import type { TaughtDetail, TaughtKind, TaughtMod, TaughtSource, TaughtState } from "@armada/settings";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { BridgeState } from "../../shared/bridge";
import { modOpened } from "./open-mod";
import { askToTell } from "./tell";
import { teachFlavour } from "./teach";

type Facts = Pick<BridgeState, "mods">;

/** What the list is read through: `window.armada` in the app, a fake in a test. */
export type FleetTaught = {
  state: () => Promise<Facts>;
  subscribe: (on: (facts: Facts) => void) => () => void;
  setModEnabled: (name: string, enabled: boolean) => Promise<Outcome>;
  promoteMod: (name: string) => Promise<Outcome>;
};

const KINDS: readonly TaughtKind[] = ["words", "fleet", "tweak", "tracker"];
const isTaught = (kind: ModSummary["kind"]): kind is TaughtKind => (KINDS as readonly (string | undefined)[]).includes(kind);

/** A window that lists no taught mod: safe mode, which reads no mod. */
export const NO_TAUGHT: TaughtSource = {
  get: () => ({ mods: [], details: {} }),
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
  const listeners = new Set<() => void>();

  const build = (): TaughtState => {
    const flavour = teachFlavour();
    const mods = rows.flatMap((row): TaughtMod[] =>
      !isTaught(row.kind)
        ? []
        : [
            {
              name: row.name,
              kind: row.kind,
              description: row.valid ? (row.description ?? "") : (row.reason ?? "Did not pass its checks"),
              on: row.taught_on ?? "",
              enabled: switched.get(row.name) ?? row.enabled,
              ...(branches.has(row.name) ? { branch: branches.get(row.name)! } : {}),
              parts: flavour?.mod?.(row)?.parts ?? [],
            },
          ],
    );
    const details: Record<string, TaughtDetail> = {};
    for (const row of rows) {
      if (row.kind === "layout" && row.taught_on !== undefined) details[row.name] = { on: row.taught_on, parts: flavour?.mod?.(row)?.parts ?? [] };
    }
    const named = modOpened.get();
    return { mods, details, ...(named !== undefined && (details[named] !== undefined || mods.some((one) => one.name === named)) ? { opened: named } : {}) };
  };

  let held = build();
  const refresh = () => {
    held = build();
    listeners.forEach((on) => on());
  };

  const ingest = (next: Facts) => {
    const key = JSON.stringify(next.mods);
    if (key === seen) return;
    seen = key;
    rows = next.mods?.mods ?? [];
    switched.clear();
    refresh();
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
