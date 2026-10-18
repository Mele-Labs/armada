// Flight control, edited in place: the tiles of one set stay where they are and turn into things to arrange. Drag one to reorder, or pick it
// up with Space and move it with the arrows (Alt and an arrow moves the tile under focus without picking it up). Delete or Backspace hides it
// and again brings it back, Enter opens its sheet, and Esc leaves without saving; Done saves. The `+` tile adds an action. A tile of the
// owner's own is edited the same way, and a built-in's sheet has Reset. The owner's ask of 18 Oct 2026. What is kept and in what shape is
// `flight-controls.ts`; what a tile does when pressed is not here.

import { useEffect, useRef, useState, type CSSProperties, type KeyboardEvent } from "react";
import { createPortal } from "react-dom";
import { Plus } from "lucide-react";
import { Button, Kbd, Tooltip } from "@armada/components";

import { FlightActionSheet } from "./FlightActionSheet";
import { entriesOf, hiddenIn, movedBy, movedTo, removedFrom, replacedIn, resetIn, saveFlight, tilesOf, useFlightFile, type FlightEntry, type FlightFile, type FlightSet } from "./flight-controls";
import { glyphOf } from "./flight-glyphs";

const ADD = "+";

/** `host` is the panel band's slot for Done and Cancel. `onClose` leaves edit mode, saved or not. */
export function FlightEdit({ set, host, onClose }: { set: FlightSet; host: HTMLElement | null; onClose: () => void }) {
  const file = useFlightFile();
  const [entries, setEntries] = useState<FlightEntry[]>(() => entriesOf(tilesOf(set, file)));
  const tiles = tilesOf(set, { ...file, [set]: entries } as FlightFile);
  const [at, setAt] = useState<string>(tiles[0]?.id ?? ADD);
  const [lifted, setLifted] = useState<string | undefined>();
  const [dragging, setDragging] = useState<string | undefined>();
  const [over, setOver] = useState<string | undefined>();
  const [sheet, setSheet] = useState<{ id: string | undefined } | undefined>();
  const [said, setSaid] = useState("");
  const list = useRef<HTMLDivElement>(null);
  const order = [...tiles.map((one) => one.id), ADD];
  const focusTile = (id: string) => window.requestAnimationFrame(() => list.current?.querySelector<HTMLElement>(`[data-edit-tile="${CSS.escape(id)}"]`)?.focus());
  // Entering edit mode puts focus on the first tile, and closing the sheet puts it back where it was.
  useEffect(() => {
    if (sheet === undefined) focusTile(at);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sheet]);
  const columns = (): number => {
    const all = [...(list.current?.querySelectorAll<HTMLElement>("[data-edit-tile]") ?? [])];
    const top = all[0]?.getBoundingClientRect().top;
    if (top === undefined) return 1;
    const first = all.findIndex((one) => Math.abs(one.getBoundingClientRect().top - top) > 2);
    return first < 0 ? all.length : Math.max(1, first);
  };
  const place = (id: string, next: readonly FlightEntry[]) => {
    const n = next.findIndex((one) => one.id === id) + 1;
    setSaid(`${tiles.find((one) => one.id === id)?.label ?? ""}, ${n} of ${next.length}`);
  };
  const shove = (id: string, by: number) => {
    const next = movedBy(entries, id, by);
    setEntries(next);
    place(id, next);
  };
  const toggleHidden = (id: string) => {
    const tile = tiles.find((one) => one.id === id);
    if (tile === undefined) return;
    setEntries(hiddenIn(entries, id, !tile.hidden));
    setSaid(`${tile.label}, ${tile.hidden ? "shown" : "hidden"}`);
  };
  const done = () => {
    saveFlight(set, entries);
    onClose();
  };
  const step = (by: number) => {
    const next = order[Math.min(order.length - 1, Math.max(0, order.indexOf(at) + by))]!;
    setAt(next);
    focusTile(next);
  };
  const key = (event: KeyboardEvent) => {
    const arrows: Record<string, number> = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: -columns(), ArrowDown: columns() };
    const by = arrows[event.key];
    const taken = (): true => (event.preventDefault(), event.stopPropagation(), true);
    if (by !== undefined && !event.metaKey && !event.ctrlKey) {
      taken();
      if (event.altKey && at !== ADD) shove(at, by);
      else if (lifted !== undefined) shove(lifted, by);
      else step(by);
      return;
    }
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    if (event.key === "Escape") {
      taken();
      if (lifted !== undefined) {
        setLifted(undefined);
        setSaid("Put down");
      } else onClose();
    } else if (event.key === " " && at !== ADD) {
      taken();
      setLifted(lifted === undefined ? at : undefined);
      setSaid(lifted === undefined ? `${tiles.find((one) => one.id === at)?.label ?? ""} picked up` : "Put down");
    } else if (event.key === "Enter") {
      taken();
      if (lifted !== undefined) setLifted(undefined);
      else setSheet({ id: at === ADD ? undefined : at });
    } else if ((event.key === "Delete" || event.key === "Backspace") && at !== ADD) {
      taken();
      toggleHidden(at);
    } else if (event.key === "Home" || event.key === "End") {
      taken();
      const next = event.key === "Home" ? order[0]! : order[order.length - 1]!;
      setAt(next);
      focusTile(next);
    }
  };
  const editing = sheet === undefined ? undefined : tiles.find((one) => one.id === sheet.id);
  return (
    <>
      {host === null
        ? null
        : createPortal(
            <>
              <Tooltip label="Leave without saving" shortcut="Esc">
                <Button variant="ghost" size="sm" onClick={onClose}>
                  Cancel
                </Button>
              </Tooltip>
              <Button variant="secondary" size="sm" onClick={done}>
                Done
              </Button>
            </>,
            host,
          )}
      <div className="deck-controls__block deck-flight-edit" data-subject="edit" data-flight-editing="" style={{ "--count": order.length, "--cols": Math.ceil(order.length / 2) } as CSSProperties}>
        <div ref={list} className="deck-controls__tiles" role="listbox" aria-label="Flight controls, editing" onKeyDown={key} onKeyUp={(event) => event.key === " " && event.preventDefault()}>
          {tiles.map((tile) => {
            const Glyph = glyphOf(tile.glyph, "zap");
            return (
              <Tooltip key={tile.id} label={tile.hidden ? `${tile.label}, hidden` : tile.label} asChild>
                <button
                  type="button"
                  role="option"
                  aria-selected={at === tile.id}
                  aria-label={tile.label}
                  className="deck-tile"
                  data-edit-tile={tile.id}
                  data-hidden={tile.hidden || undefined}
                  data-lifted={lifted === tile.id || undefined}
                  data-over={(over === tile.id && dragging !== tile.id) || undefined}
                  tabIndex={at === tile.id ? 0 : -1}
                  draggable
                  onFocus={() => setAt(tile.id)}
                  onClick={() => {
                    setAt(tile.id);
                    setSheet({ id: tile.id });
                  }}
                  onDragStart={(event) => {
                    event.dataTransfer.effectAllowed = "move";
                    event.dataTransfer.setData("text/plain", tile.id);
                    setDragging(tile.id);
                  }}
                  onDragOver={(event) => {
                    if (dragging === undefined) return;
                    event.preventDefault();
                    setOver(tile.id);
                  }}
                  onDrop={(event) => {
                    event.preventDefault();
                    if (dragging !== undefined) setEntries(movedTo(entries, dragging, tile.id));
                    setDragging(undefined);
                    setOver(undefined);
                  }}
                  onDragEnd={() => {
                    setDragging(undefined);
                    setOver(undefined);
                  }}
                >
                  <Glyph size={20} aria-hidden="true" />
                  <span className="deck-tile__text">{tile.text}</span>
                  {tile.key === undefined ? null : (
                    <Kbd className="deck-tile__key" aria-hidden="true">
                      {tile.key}
                    </Kbd>
                  )}
                </button>
              </Tooltip>
            );
          })}
          <Tooltip label="Add an action" asChild>
            <button type="button" role="option" aria-selected={at === ADD} aria-label="Add action" className="deck-tile" data-edit-tile={ADD} data-add="" tabIndex={at === ADD ? 0 : -1} onFocus={() => setAt(ADD)} onClick={() => setSheet({ id: undefined })}>
              <Plus size={20} aria-hidden="true" />
            </button>
          </Tooltip>
        </div>
        <span className="deck-sr" role="status" aria-live="polite">
          {said}
        </span>
      </div>
      {sheet === undefined ? null : (
        <FlightActionSheet
          set={set}
          tile={editing}
          tiles={tiles}
          entries={entries}
          onClose={() => setSheet(undefined)}
          onSave={(entry) => {
            setEntries(editing === undefined ? [...entries, entry] : replacedIn(entries, entry));
            setAt(entry.id);
            setSheet(undefined);
          }}
          {...(editing?.builtIn !== undefined && editing.changed
            ? {
                onReset: () => {
                  setEntries(resetIn(entries, editing.id));
                  setSheet(undefined);
                },
              }
            : {})}
          {...(editing !== undefined && editing.builtIn === undefined
            ? {
                onRemove: () => {
                  setEntries(removedFrom(entries, editing.id));
                  setAt(tiles.find((one) => one.id !== editing.id)?.id ?? ADD);
                  setSheet(undefined);
                },
              }
            : {})}
        />
      )}
    </>
  );
}
