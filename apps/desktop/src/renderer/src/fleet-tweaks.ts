// The theme tweaks Bridge lays over whichever theme is chosen, over Fleet: its mod list and the stylesheet Fleet checked for each tweak
// mod that is on. **Bridge applies the `css` that `validate_mod` returned and no other text** (`docs/concepts/mods.md`), in mod name
// order. A tweak that is off, invalid or not yet read adds nothing, and one that is rewritten keeps the sheet last read until the new
// one lands, so the screen does not flicker to the theme.

import type { ModChecked } from "@armada/protocol";

import type { BridgeState } from "../../shared/bridge";

type Facts = Pick<BridgeState, "mods">;

/** One tweak mod's checked stylesheet. */
export type TweakSheet = { name: string; css: string };

/** What the tweaks are read from: `window.armada` in the app, a fake in a test. */
export type FleetTweaks = {
  state: () => Promise<Facts>;
  subscribe: (on: (facts: Facts) => void) => () => void;
  validateMod: (name: string) => Promise<ModChecked | null>;
};

export interface TweakSource {
  get(): readonly TweakSheet[];
  subscribe(on: () => void): () => void;
}

const NONE: readonly TweakSheet[] = [];

/** A window that lays nothing over its theme: safe mode, and a window that was given no source. */
export const NO_TWEAKS: TweakSource = { get: () => NONE, subscribe: () => () => undefined };

export function createFleetTweaks(fleet: FleetTweaks): TweakSource {
  let held: readonly TweakSheet[] = NONE;
  let rows: readonly { name: string; at: string }[] = [];
  let seen = "";
  const read = new Map<string, { at: string; css: string }>();
  const asking = new Set<string>();
  const listeners = new Set<() => void>();

  const refresh = () => {
    const next = rows.flatMap((row) => {
      const css = read.get(row.name)?.css;
      return css === undefined ? [] : [{ name: row.name, css }];
    });
    if (JSON.stringify(next) === JSON.stringify(held)) return;
    held = next;
    listeners.forEach((on) => on());
  };

  const ask = (row: { name: string; at: string }) => {
    const key = `${row.name}@${row.at}`;
    if (read.get(row.name)?.at === row.at || asking.has(key)) return;
    asking.add(key);
    fleet.validateMod(row.name).then(
      (checked) => {
        asking.delete(key);
        if (checked === null) return;
        // A tweak that no longer passes is not laid over the theme; the one before it is not kept.
        if (checked.valid && checked.css !== undefined) read.set(row.name, { at: row.at, css: checked.css });
        else read.delete(row.name);
        refresh();
      },
      () => void asking.delete(key),
    );
  };

  const ingest = (next: Facts) => {
    const on = (next.mods?.mods ?? [])
      .filter((row) => row.kind === "tweak" && row.valid && row.enabled)
      .map((row) => ({ name: row.name, at: row.changed_at ?? "" }))
      .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
    const key = JSON.stringify(on);
    if (key === seen) return;
    seen = key;
    rows = on;
    for (const name of [...read.keys()]) if (!on.some((row) => row.name === name)) read.delete(name);
    refresh();
    on.forEach(ask);
  };

  fleet.state().then(ingest, () => undefined);
  fleet.subscribe(ingest);

  return {
    get: () => held,
    subscribe: (on) => {
      listeners.add(on);
      return () => void listeners.delete(on);
    },
  };
}
