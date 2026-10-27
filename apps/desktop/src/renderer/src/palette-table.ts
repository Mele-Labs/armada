// What Settings → Command palette reads and changes: the rows that hold an alias, a hotkey or a pin, named from the rows
// the palette last listed, and the same refusals the palette's own panel gives a word or a key.

import { parseChord, refuseAlias, refuseHotkey } from "@armada/components";
import type { PaletteSettingsProps } from "@armada/settings";
import { setAlias, setHotkey, setOpens, togglePin, usePaletteMemory } from "./palette-memory";
import { useKnownRows } from "./palette-known";

export function usePaletteTable(): PaletteSettingsProps {
  const memory = usePaletteMemory();
  const known = useKnownRows();
  const named = new Map(known.map((one) => [one.id, one]));
  const holding = new Set([...memory.pinned, ...Object.keys(memory.aliases), ...Object.keys(memory.hotkeys)]);
  const rows = [...holding].flatMap((id) => {
    const row = named.get(id);
    return row === undefined
      ? []
      : [{ id, name: row.label, kind: row.kind, alias: memory.aliases[id] ?? "", hotkey: memory.hotkeys[id] ?? "", pinned: memory.pinned.includes(id), keyable: row.keyable }];
  });
  rows.sort((a, b) => a.name.localeCompare(b.name));

  return {
    opens: memory.opens,
    onOpens: setOpens,
    rows,
    onPin: togglePin,
    onAlias: (id, word) => {
      const why = word === "" ? undefined : refuseAlias(word, Object.entries(memory.aliases).flatMap(([other, alias]) => (other === id ? [] : [{ label: named.get(other)?.label ?? other, alias }])));
      if (why === undefined) setAlias(id, word);
      return why;
    },
    onHotkey: (id, spelled) => {
      const chord = spelled === "" ? null : parseChord(spelled);
      const why = chord === null ? undefined : refuseHotkey(chord, known.flatMap((one) => (one.id === id || one.shortcut === undefined ? [] : [{ label: one.label, hotkey: one.shortcut }])));
      if (why === undefined) setHotkey(id, spelled);
      return why;
    },
  };
}
