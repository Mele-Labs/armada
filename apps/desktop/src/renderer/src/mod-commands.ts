// The rows Commands mods add to the palette, over Fleet: the mods that are on and valid, and the `commands` Fleet checked for
// each (`validate_mod`). **Bridge draws the rows from that answer and no other text** (`docs/concepts/mods.md`). A row's palette id is
// `cmd:<mod name>:<slug of its name>`; the id held here is that one without its `cmd:`, as the palette's own command ids are.

import { useEffect, useState } from "react";
import { parseChord, refuseHotkey } from "@armada/components";
import type { CommandsFile, ModChecked, PaletteCommand as ModCommand } from "@armada/protocol";
import type { PaletteCommand } from "@armada/shell";

import type { BridgeApi } from "../../shared/api";
import type { BridgeState } from "../../shared/bridge";
import { withState } from "./state-feed";
import { skipsMods } from "./theme-loader";

/** One row of a Commands mod. */
export type ModRow = { id: string; mod: string; command: ModCommand };

/** `name` as Fleet spells it in a row's id: lower case, each run of anything but letters and digits one `-`, none at either end. */
export const slugOf = (name: string): string =>
  name
    .toLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, "-")
    .replace(/^-+|-+$/g, "");

export const rowsOf = (mod: string, file: CommandsFile): ModRow[] => file.command.map((command) => ({ id: `${mod}:${slugOf(command.name)}`, mod, command }));

/** What the palette draws of a row. `hotkey` is the key it was allowed, which is not always the one the file names. */
export function displayOf(row: ModRow, hotkey: string | undefined): PaletteCommand {
  const { command } = row;
  const kind = command.prompt !== undefined ? "prompt" : command.url !== undefined ? "url" : command.shell !== undefined ? "shell" : "chain";
  const value =
    kind === "url" ? command.url!.replace(/^https?:\/\//, "") : kind === "shell" ? command.shell! : kind === "prompt" && command.blanks !== undefined ? command.blanks.map((one) => `{${one}}`).join(" ") : undefined;
  return {
    id: row.id,
    label: command.name,
    kind,
    ...(value === undefined ? {} : { value }),
    ...(command.aliases === undefined || command.aliases.length === 0 ? {} : { aliases: command.aliases }),
    ...(hotkey === undefined ? {} : { hotkey }),
  };
}

/**
 * The keys the rows' files name that are allowed, by palette row id. **A key he recorded for a row wins over the file's**, and a key
 * is refused as a recorded one is (`refuseHotkey`): by ⌘ or ⌃ missing, an act, the system, or a row that already has it. Recorded keys
 * hold their place first, then the files' in row order.
 */
export function keysOf(rows: readonly ModRow[], recorded: Readonly<Record<string, string>>): Record<string, string> {
  const held = Object.entries(recorded).map(([id, hotkey]) => ({ label: id, hotkey }));
  const allowed: Record<string, string> = {};
  for (const row of rows) {
    const id = `cmd:${row.id}`;
    const spelled = row.command.hotkey;
    if (spelled === undefined || recorded[id] !== undefined) continue;
    const chord = parseChord(spelled);
    if (chord === null || refuseHotkey(chord, held) !== undefined) continue;
    allowed[id] = spelled;
    held.push({ label: id, hotkey: spelled });
  }
  return allowed;
}

/** The text with each blank it holds filled from `values`. A brace after a `$` is shell text, and a blank with no value is left. */
export const fill = (text: string, values: Readonly<Record<string, string>>): string =>
  text.replace(/(?<!\$)\{([a-z][a-z0-9_]*)\}/g, (whole, name: string) => values[name] ?? whole);

type Facts = Pick<BridgeState, "mods">;

/** The rows of every Commands mod that is on, read as Fleet's mod list changes and again when a mod's files are rewritten. */
export function useModRows(): readonly ModRow[] {
  const [rows, setRows] = useState<readonly ModRow[]>([]);
  useEffect(() => {
    const api = window.armada as Partial<BridgeApi> | undefined;
    // A test's fake window with no state to follow, and safe mode, which reads no mods.
    if (skipsMods() || typeof api?.syncState !== "function" || typeof api.subscribeChanges !== "function" || typeof api.validateMod !== "function") return;
    const fleet = withState(api as BridgeApi);
    const read = new Map<string, { at: string; rows: ModRow[] }>();
    let on: { name: string; at: string }[] = [];
    let seen = "";
    let live = true;
    const publish = () => {
      if (!live) return;
      const next = on.flatMap((one) => read.get(one.name)?.rows ?? []);
      setRows((was) => (JSON.stringify(was) === JSON.stringify(next) ? was : next));
    };
    const ask = (one: { name: string; at: string }) => {
      if (read.get(one.name)?.at === one.at) return;
      void fleet.validateMod(one.name).then(
        (checked: ModChecked | null) => {
          if (!live || checked === null) return;
          // A mod that no longer passes adds nothing; the rows before it are not kept.
          if (checked.valid && checked.commands !== undefined) read.set(one.name, { at: one.at, rows: rowsOf(one.name, checked.commands) });
          else read.delete(one.name);
          publish();
        },
        () => undefined,
      );
    };
    const ingest = (state: Facts) => {
      if (!live) return;
      const next = (state.mods?.mods ?? [])
        .filter((row) => row.kind === "commands" && row.valid && row.enabled)
        .map((row) => ({ name: row.name, at: row.changed_at ?? "" }))
        .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
      const key = JSON.stringify(next);
      if (key === seen) return;
      seen = key;
      on = next;
      for (const name of [...read.keys()]) if (!next.some((one) => one.name === name)) read.delete(name);
      publish();
      next.forEach(ask);
    };
    void fleet.state().then(ingest, () => undefined);
    const off = fleet.subscribe(ingest);
    return () => {
      live = false;
      off();
    };
  }, []);
  return rows;
}
