// The layout Bridge draws, over Fleet: its layout mods, the `layout.json` Fleet checked for each,
// and the owner's own choices kept in the `layout_choices` preference. A `LayoutSource` like the
// mock's, so every surface reads it unchanged. `docs/concepts/layout-mods.md`.
//
// **Bridge applies the text Fleet returned and no other**, parsed by the same `parseLayout` Fleet's
// rules mirror. A mod that is off, invalid or not yet read adds nothing, so what is drawn is the
// registry's defaults until a layer says otherwise, and the owner's own choice is the last layer.

import type { ModChecked, ModSummary, Outcome, SavePreference } from "@armada/protocol";
import { parseLayout, serializeLayout } from "@armada/shell";
import type { LayoutFile, LayoutMod, LayoutRegion, LayoutSource, LayoutState, RegionChoice } from "@armada/shell";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { BridgeState } from "../../shared/bridge";
import { askToTell } from "./tell";

type Facts = Pick<BridgeState, "mods" | "preferences">;

/** What Bridge reaches Fleet through: `window.armada` in the app, a fake in a test. */
export type FleetLayout = {
  state: () => Promise<Facts>;
  subscribe: (on: (facts: Facts) => void) => () => void;
  validateMod: (name: string) => Promise<ModChecked | null>;
  setModEnabled: (name: string, enabled: boolean) => Promise<Outcome>;
  promoteMod: (name: string) => Promise<Outcome>;
  savePreference: (save: SavePreference) => Promise<Outcome>;
};

const NONE: LayoutFile = { regions: {} };

type Read = { at: string; file: LayoutFile; problem?: string };

export function createFleetLayout(fleet: FleetLayout, tell: (sentence: string) => void = askToTell): LayoutSource {
  let rows: readonly ModSummary[] = [];
  // The owner's choices as Fleet last said them, and as he has pressed them since.
  let own: LayoutFile = NONE;
  let saving = 0;
  let seen = "";
  const reads = new Map<string, Read>();
  const asking = new Set<string>();
  const branches = new Map<string, string>();
  // A switch just pressed, drawn at once; Fleet's next list replaces it.
  const switched = new Map<string, boolean>();
  const listeners = new Set<() => void>();

  const modsNow = (): LayoutMod[] =>
    rows
      .filter((row) => row.kind === "layout")
      .map((row) => {
        // A mod rewritten keeps the file last read until the new one lands, so the screen does not flicker to defaults.
        const current = reads.get(row.name);
        const problem = row.valid ? current?.problem : (row.reason ?? "Did not pass its checks");
        return {
          name: row.name,
          title: row.name,
          enabled: switched.get(row.name) ?? row.enabled,
          ...(problem === undefined ? {} : { problem }),
          ...(branches.has(row.name) ? { branch: branches.get(row.name)! } : {}),
          file: current?.file ?? NONE,
        };
      });

  let held: LayoutState = { mods: [], own };
  const refresh = () => {
    held = { mods: modsNow(), own };
    listeners.forEach((on) => on());
  };

  /** Ask Fleet for the text of each layout mod that is on and passes, once per `changed_at`. */
  const read = () => {
    for (const row of rows) {
      const at = row.changed_at ?? "";
      const key = `${row.name}@${at}`;
      if (row.kind !== "layout" || !row.valid || !row.enabled || reads.get(row.name)?.at === at || asking.has(key)) continue;
      asking.add(key);
      fleet.validateMod(row.name).then(
        (checked) => {
          asking.delete(key);
          if (checked === null) return;
          if (!checked.valid || checked.layout === undefined) reads.set(row.name, { at, file: NONE, problem: checked.problems[0] ?? "Did not pass its checks" });
          else {
            const parsed = parseLayout(checked.layout);
            reads.set(row.name, { at, file: parsed.file, ...(parsed.problems.length === 0 ? {} : { problem: parsed.problems[0]! }) });
          }
          refresh();
        },
        () => void asking.delete(key),
      );
    }
  };

  /** Save the owner's choices. A refusal puts the old ones back and says why. */
  const keep = async (next: LayoutFile): Promise<void> => {
    const before = own;
    own = next;
    saving += 1;
    refresh();
    const outcome = await fleet.savePreference({ name: "layout_choices", value: false, text: serializeLayout(next) });
    saving -= 1;
    if (outcome.ok) return;
    if (own === next) {
      own = before;
      refresh();
    }
    tell(refusalWords(outcome));
  };

  const ingest = (next: Facts) => {
    const choices = next.preferences.layout_choices ?? "";
    const key = JSON.stringify([next.mods, choices]);
    if (key === seen) return;
    seen = key;
    rows = next.mods?.mods ?? [];
    switched.clear();
    // Text that does not parse is no choices, so the screen is the mods' and the defaults.
    if (saving === 0) own = choices === "" ? NONE : parseLayout(choices).file;
    refresh();
    read();
  };

  const setEnabled = (name: string, enabled: boolean): void => {
    switched.set(name, enabled);
    refresh();
    void fleet.setModEnabled(name, enabled).then((outcome) => {
      if (outcome.ok) return;
      switched.delete(name);
      refresh();
      tell(refusalWords(outcome));
    });
  };

  fleet.state().then(ingest, () => undefined);
  fleet.subscribe(ingest);

  return {
    get: () => held,
    subscribe: (on) => {
      listeners.add(on);
      return () => void listeners.delete(on);
    },
    setOwn: (region: LayoutRegion, choice: RegionChoice | undefined) => {
      const { [region]: _was, ...rest } = own.regions;
      void keep({ regions: choice === undefined ? rest : { ...rest, [region]: choice } });
    },
    toDefaults: () => {
      void keep(NONE);
      for (const mod of modsNow()) {
        if (mod.enabled) setEnabled(mod.name, false);
      }
    },
    setEnabled,
    promote: (name) => {
      void fleet.promoteMod(name).then((outcome) => {
        if (!outcome.ok) return tell(refusalWords(outcome));
        if (outcome.modPromoted === undefined) return;
        branches.set(name, outcome.modPromoted.branch);
        refresh();
      });
    },
  };
}
