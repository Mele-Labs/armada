import { useEffect, useState } from "react";
import { X } from "lucide-react";

import { chordOf, formatChord } from "../../keymap";
import { Button } from "../../primitives/Button/Button";
import { Checkbox } from "../../primitives/Checkbox/Checkbox";
import { Input } from "../../primitives/Input/Input";
import { KbdChord } from "../../primitives/Kbd/Kbd";
import { Table, TableBody, TableCell, TableHead, TableHeaderCell, TableRow } from "../../primitives/Table/Table";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/** A row of the command palette that holds a word, a key or a pin, as Settings lists it. */
export type PaletteRowView = {
  id: string;
  name: string;
  kind: string;
  /** The word he gave it, or `""`. */
  alias: string;
  /** The key he gave it, spelled as the palette draws keys, or `""`. */
  hotkey: string;
  pinned: boolean;
  /** False for a row that ends work, which takes no key. */
  keyable: boolean;
};

/**
 * Every row of the palette that holds an alias, a hotkey or a pin, each cell editable where it stands. **Alias is a
 * field**: ↵ keeps it, Esc puts it back, and emptying it lets it go. **Hotkey is a control that takes the next key
 * pressed**, as Settings → Keyboard's are: Esc lets go, Delete unbinds. Each cell that holds something has its clear in
 * a column of its own width, so the clears line up down the table. A word or key `onAlias` or `onHotkey` refuses is
 * said under its cell and kept for another try.
 */
export type PaletteRowsProps = {
  rows: readonly PaletteRowView[];
  /** The word given, `""` to let it go. Returns why it is refused, or nothing once kept. */
  onAlias: (id: string, word: string) => string | undefined;
  /** The key pressed, as the registry spells it, `""` to let it go. Returns why it is refused, or nothing once kept. */
  onHotkey: (id: string, spelled: string) => string | undefined;
  onPin: (id: string) => void;
};

export function PaletteRows({ rows, onAlias, onHotkey, onPin }: PaletteRowsProps) {
  // One key waits for a press at a time, in the whole table, and the reason the last one was refused.
  const [listening, setListening] = useState<string | null>(null);
  const [refused, setRefused] = useState<string | null>(null);

  useEffect(() => {
    if (listening === null) return;
    const id = listening;
    // The capture phase and stopped there, so nothing under Settings acts on the key being given.
    const pressed = (event: KeyboardEvent): void => {
      event.preventDefault();
      event.stopImmediatePropagation();
      if (event.key === "Escape") return stop();
      if (event.key === "Backspace" || event.key === "Delete") return void (onHotkey(id, ""), stop());
      const chord = chordOf(event);
      if (chord === null) return;
      const why = onHotkey(id, formatChord(chord));
      if (why === undefined) stop();
      else setRefused(why);
    };
    const stop = () => (setListening(null), setRefused(null));
    window.addEventListener("keydown", pressed, true);
    return () => window.removeEventListener("keydown", pressed, true);
  }, [listening, onHotkey]);

  // ↑ ↓ move to the same control in the row above or below, since a column of them is what is being driven.
  function onKeyDown(event: React.KeyboardEvent<HTMLTableElement>): void {
    if (listening !== null || (event.key !== "ArrowUp" && event.key !== "ArrowDown")) return;
    const here = (event.target as HTMLElement).closest<HTMLElement>("[data-col]");
    const col = here?.dataset["col"];
    if (here === undefined || here === null || col === undefined) return;
    const column = Array.from(event.currentTarget.querySelectorAll<HTMLElement>(`[data-col="${col}"]`));
    const next = column[column.indexOf(here) + (event.key === "ArrowDown" ? 1 : -1)];
    if (next === undefined) return;
    event.preventDefault();
    next.focus();
  }

  return (
    <Table className="armada-palette-rows" aria-label="Command palette rows" onKeyDown={onKeyDown}>
      <colgroup>
        <col className="armada-palette-rows__name" />
        <col className="armada-palette-rows__kind" />
        <col className="armada-palette-rows__alias" />
        <col className="armada-palette-rows__hotkey" />
        <col className="armada-palette-rows__pinned" />
      </colgroup>
      <TableHead>
        <TableRow>
          <TableHeaderCell>Name</TableHeaderCell>
          <TableHeaderCell>Kind</TableHeaderCell>
          <TableHeaderCell>Alias</TableHeaderCell>
          <TableHeaderCell>Hotkey</TableHeaderCell>
          <TableHeaderCell>Pinned</TableHeaderCell>
        </TableRow>
      </TableHead>
      <TableBody>
        {rows.map((row) => (
          <TableRow key={row.id}>
            <TableCell variant="primary" className="armada-palette-rows__text">
              {row.name}
            </TableCell>
            <TableCell variant="secondary" className="armada-palette-rows__text">
              {row.kind}
            </TableCell>
            <TableCell>
              <AliasCell row={row} onAlias={onAlias} />
            </TableCell>
            <TableCell>
              <div className="armada-palette-rows__cell">
                {row.keyable ? (
                  <button
                    type="button"
                    className="armada-palette-rows__key"
                    data-col="hotkey"
                    data-recording={listening === row.id ? "" : undefined}
                    data-refused={listening === row.id && refused !== null ? "" : undefined}
                    aria-pressed={listening === row.id}
                    aria-label={listening === row.id ? `${row.name}: press a key, Esc to cancel, Delete to clear` : `${row.name}, hotkey${row.hotkey === "" ? "" : `: ${row.hotkey}`}`}
                    onClick={() => (setRefused(null), setListening(listening === row.id ? null : row.id))}
                  >
                    {listening === row.id ? <span className="armada-palette-rows__waiting">Press a key</span> : row.hotkey === "" ? null : <KbdChord keys={Array.from(row.hotkey)} aria-hidden />}
                  </button>
                ) : null}
                <Clear name={row.name} what="hotkey" col="hotkey-clear" shown={row.hotkey !== ""} onClear={() => void onHotkey(row.id, "")} />
              </div>
              {listening === row.id && refused !== null ? (
                <span className="armada-palette-rows__refused" role="alert">
                  {refused}
                </span>
              ) : null}
            </TableCell>
            <TableCell>
              <Checkbox checked={row.pinned} data-col="pinned" onChange={() => onPin(row.id)}>
                <span className="armada-palette-rows__hidden">{`Pinned: ${row.name}`}</span>
              </Checkbox>
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}

/** The cell's clear, in a slot that keeps its width whether or not there is something to clear. */
function Clear({ name, what, col, shown, onClear }: { name: string; what: string; col: string; shown: boolean; onClear: () => void }) {
  return (
    <span className="armada-palette-rows__clear">
      {shown ? (
        <Tooltip label={`Clear ${what}`}>
          <Button variant="ghost" size="sm" iconOnly data-col={col} aria-label={`Clear ${what}: ${name}`} onClick={onClear}>
            <X size={12} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      ) : null}
    </span>
  );
}

function AliasCell({ row, onAlias }: { row: PaletteRowView; onAlias: PaletteRowsProps["onAlias"] }) {
  // What is typed and not yet kept, or `null` while the field shows what is kept.
  const [typed, setTyped] = useState<string | null>(null);
  const [refused, setRefused] = useState<string | null>(null);

  function keep(): void {
    if (typed === null) return;
    const word = typed.trim();
    if (word === row.alias) return void (setTyped(null), setRefused(null));
    const why = onAlias(row.id, word);
    if (why === undefined) setTyped(null);
    setRefused(why ?? null);
  }

  return (
    <div className="armada-palette-rows__cell">
      <Input
        aria-label={`Alias: ${row.name}`}
        data-col="alias"
        value={typed ?? row.alias}
        invalid={refused !== null}
        {...(refused === null ? {} : { message: refused })}
        onChange={(event) => (setTyped(event.target.value), setRefused(null))}
        onBlur={keep}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            keep();
          } else if (event.key === "Escape" && typed !== null) {
            event.preventDefault();
            event.stopPropagation();
            setTyped(null);
            setRefused(null);
          }
        }}
      />
      <Clear name={row.name} what="alias" col="alias-clear" shown={row.alias !== "" && typed === null} onClear={() => void onAlias(row.id, "")} />
    </div>
  );
}
