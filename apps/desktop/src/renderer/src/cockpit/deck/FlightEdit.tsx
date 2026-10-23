// Flight control, edited in place: the tiles of one set stay where they are and turn into things to arrange. Drag one to reorder, or pick it
// up with Space and move it with the arrows (Alt and an arrow moves the tile under focus without picking it up). Delete or Backspace hides it
// and again brings it back, Enter opens its sheet, and Esc leaves without saving; Done saves. The `+` tile adds an action. A tile of the
// owner's own is edited the same way, and a built-in's sheet has Reset. The owner's ask of 18 Oct 2026. What is kept and in what shape is
// `flight-controls.ts`; what a tile does when pressed is not here.
//
// **The open repository's actions are a group at the end**, moved only among themselves, and Done saves each file that changed. A save to
// the repository that fails keeps the edit on screen and says why.

import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { createPortal } from "react-dom";
import { Plus } from "lucide-react";
import { Eclipse } from "lucide-react";
import { Button, Kbd, Tooltip } from "@armada/components";

import { FlightActionSheet } from "./FlightActionSheet";
import { flightGrid, pageCell, pagesOf } from "./flight-pages";
import { FlightBlock } from "./FlightBlock";
import {
  NOTHING_CHANGED,
  committedFileOf,
  entriesOf,
  hiddenIn,
  movedBy,
  movedTo,
  removedFrom,
  replacedIn,
  repositoryFileOf,
  resetIn,
  saveFlight,
  saveRepositoryFlight,
  tilesOf,
  useFlightFile,
  useRepositoryFlight,
  type FlightEntry,
  type FlightFile,
  type FlightSet,
} from "./flight-controls";
import { glyphOf } from "./flight-glyphs";
import { tileCap } from "./session-tiles";

const ADD = "+";

/** The repository's own actions of one set, as Edit holds them. */
const theirsOf = (set: FlightSet, file: FlightFile | undefined): FlightEntry[] | undefined => (file === undefined ? undefined : entriesOf(tilesOf(set, NOTHING_CHANGED, file).filter((one) => one.fromRepository)));
const same = (one: readonly FlightEntry[] | undefined, other: readonly FlightEntry[] | undefined): boolean => JSON.stringify(one) === JSON.stringify(other);

/** `host` is the panel band's slot for Done and Cancel. `onClose` leaves edit mode, saved or not. `repository` is the one open, if any, and `sessionId` the Job open in it. */
export function FlightEdit({ set, host, repository, sessionId, onClose }: { set: FlightSet; host: HTMLElement | null; repository?: { root: string; name?: string } | undefined; sessionId?: string | undefined; onClose: () => void }) {
  const file = useFlightFile();
  const flight = useRepositoryFlight(repository?.root, sessionId);
  // The repository's committed tiles are not the person's to arrange; they are read only to mark one of his that hides one.
  const committed = committedFileOf(flight);
  const theirFile = repositoryFileOf(flight);
  const [entries, setEntries] = useState<FlightEntry[]>(() => entriesOf(tilesOf(set, file)));
  /** Absent until the repository's file is read, and while none is open. */
  const [theirs, setTheirs] = useState<FlightEntry[] | undefined>(() => theirsOf(set, theirFile));
  // A file read after Edit opened is taken once; an edit already made is not replaced.
  useEffect(() => {
    if (theirs === undefined && theirFile !== undefined) setTheirs(theirsOf(set, theirFile));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [theirFile]);
  const [failed, setFailed] = useState<string | undefined>();
  const named = repository === undefined ? "" : (repository.name ?? repository.root);
  const merged = tilesOf(set, { ...file, [set]: entries } as FlightFile, theirs === undefined || theirFile === undefined ? undefined : { ...theirFile, [set]: theirs }, committed).filter((one) => !one.committed);
  const isTheirs = (id: string): boolean => theirs?.some((one) => one.id === id) === true;
  // The repository's sit together at the end here, wherever Pause is drawn outside edit mode.
  const tiles = [...merged.filter((one) => !one.fromRepository), ...merged.filter((one) => one.fromRepository)];
  const [at, setAt] = useState<string>(tiles[0]?.id ?? ADD);
  const [lifted, setLifted] = useState<string | undefined>();
  const [dragging, setDragging] = useState<string | undefined>();
  const [over, setOver] = useState<string | undefined>();
  const [sheet, setSheet] = useState<{ id: string | undefined } | undefined>();
  const [said, setSaid] = useState("");
  const list = useRef<HTMLDivElement>(null);
  const order = [...tiles.map((one) => one.id), ADD];
  // Pages are laid out over every cell drawn, the hidden tiles and `+` too, which is the last cell of the last page.
  const laid = tiles.length + 1;
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
  /** The list `id` is in, changed by `change`. */
  const within = (id: string, change: (list: readonly FlightEntry[]) => FlightEntry[]): FlightEntry[] => {
    if (theirs !== undefined && isTheirs(id)) {
      const next = change(theirs);
      setTheirs(next);
      return next;
    }
    const next = change(entries);
    setEntries(next);
    return next;
  };
  const shove = (id: string, by: number) => place(id, within(id, (list) => movedBy(list, id, by)));
  const toggleHidden = (id: string) => {
    const tile = tiles.find((one) => one.id === id);
    if (tile === undefined) return;
    within(id, (list) => hiddenIn(list, id, !tile.hidden));
    setSaid(`${tile.label}, ${tile.hidden ? "shown" : "hidden"}`);
  };
  const done = async () => {
    setFailed(undefined);
    if (!same(entries, entriesOf(tilesOf(set, file)))) void saveFlight(set, entries);
    if (repository !== undefined && theirs !== undefined && !same(theirs, theirsOf(set, theirFile))) {
      const saved = await saveRepositoryFlight(repository.root, set, theirs);
      if ("said" in saved) {
        setFailed(`Actions in ${named} not saved: ${saved.said}`);
        return;
      }
    }
    onClose();
  };
  const key = (event: KeyboardEvent) => {
    // The tile the key is for: the one it was sent to, which is the one in focus when a person types it.
    const here = (event.target as HTMLElement).closest<HTMLElement>("[data-edit-tile]")?.dataset.editTile ?? at;
    const arrows: Record<string, number> = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: -columns(), ArrowDown: columns() };
    const by = arrows[event.key];
    const taken = (): true => (event.preventDefault(), event.stopPropagation(), true);
    if (by !== undefined && !event.metaKey && !event.ctrlKey) {
      taken();
      if (event.altKey && here !== ADD) shove(here, by);
      else if (lifted !== undefined) shove(lifted, by);
      else {
        const next = order[Math.min(order.length - 1, Math.max(0, order.indexOf(here) + by))]!;
        setAt(next);
        focusTile(next);
      }
      return;
    }
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    if (event.key === "Escape") {
      taken();
      if (lifted !== undefined) {
        setLifted(undefined);
        setSaid("Put down");
      } else onClose();
    } else if (event.key === " " && here !== ADD) {
      taken();
      setLifted(lifted === undefined ? here : undefined);
      setSaid(lifted === undefined ? `${tiles.find((one) => one.id === here)?.label ?? ""} picked up` : "Put down");
    } else if (event.key === "Enter") {
      taken();
      if (lifted !== undefined) setLifted(undefined);
      else setSheet({ id: here === ADD ? undefined : here });
    } else if ((event.key === "Delete" || event.key === "Backspace") && here !== ADD) {
      taken();
      toggleHidden(here);
    } else if (event.key === "Home" || event.key === "End") {
      taken();
      const next = event.key === "Home" ? order[0]! : order[order.length - 1]!;
      setAt(next);
      focusTile(next);
    }
  };
  const editing = sheet === undefined ? undefined : tiles.find((one) => one.id === sheet.id);
  // Over the whole window and not inside the panel, whose edge would clip it.
  const adding = (
    <FlightActionSheet
      set={set}
      tile={editing}
      tiles={tiles}
      entries={[...entries, ...(theirs ?? [])]}
      onClose={() => setSheet(undefined)}
      onSave={(entry, only) => {
        const put = (list: readonly FlightEntry[]) => (list.some((one) => one.id === entry.id) ? replacedIn(list, entry) : [...list, entry]);
        if (only && theirs !== undefined) {
          setEntries(removedFrom(entries, entry.id));
          setTheirs(put(theirs));
        } else {
          setTheirs(theirs === undefined ? undefined : removedFrom(theirs, entry.id));
          setEntries(put(entries));
        }
        setAt(entry.id);
        setSheet(undefined);
      }}
      {...(theirs === undefined ? {} : { only: { name: named, on: editing !== undefined && isTheirs(editing.id) } })}
      {...(repository !== undefined && editing !== undefined && theirFile?.[set].some((one) => one.id === editing.id) === true ? { kept: { root: repository.root } } : {})}
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
              within(editing.id, (list) => removedFrom(list, editing.id));
              setAt(tiles.find((one) => one.id !== editing.id)?.id ?? ADD);
              setSheet(undefined);
            },
          }
        : {})}
    />
  );
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
              <Button variant="secondary" size="sm" onClick={() => void done()}>
                Done
              </Button>
            </>,
            host,
          )}
      <FlightBlock
        pages={set === "job" ? pagesOf(laid).count : undefined}
        className="deck-controls__block deck-flight-edit"
        data-flight-editing=""
        {...(set === "job" ? { "data-subject": "edit", style: flightGrid(laid) } : {})}
      >
        <div ref={list} className={set === "job" ? "deck-controls__tiles" : "deck-flight-edit__flat"} role="listbox" aria-label="Flight controls, editing" onKeyDown={key} onKeyUp={(event) => event.key === " " && event.preventDefault()}>
          {tiles.map((tile, cell) => {
            const Glyph = glyphOf(tile.glyph, "zap");
            return (
              <Tooltip key={tile.id} label={[tile.label, ...(tile.fromRepository ? [`only in ${named}`] : []), ...(tile.hidden ? ["hidden"] : []), ...(tile.hides === undefined ? [] : [`hides the repository's ${tile.hides}`])].join(", ")} asChild>
                <button
                  type="button"
                  role="option"
                  aria-selected={at === tile.id}
                  aria-label={tile.label}
                  className="deck-tile"
                  {...pageCell(cell, laid)}
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
                    // A drop moves a tile only among those kept in the same file.
                    if (dragging !== undefined && isTheirs(dragging) === isTheirs(tile.id)) within(dragging, (list) => movedTo(list, dragging, tile.id));
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
                  {tile.hides === undefined ? null : (
                    <span className="deck-tile__hides" role="img" aria-label={`Hides the repository's ${tile.hides}`}>
                      <Eclipse size={12} strokeWidth={2.5} aria-hidden="true" />
                    </span>
                  )}
                  {tile.key === undefined ? null : (
                    <Kbd className="deck-tile__key" aria-hidden="true">
                      {tileCap(tile.key)}
                    </Kbd>
                  )}
                </button>
              </Tooltip>
            );
          })}
          <Tooltip label="Add an action" asChild>
            <button type="button" role="option" aria-selected={at === ADD} aria-label="Add action" className="deck-tile" {...pageCell(tiles.length, laid)} data-edit-tile={ADD} data-add="" tabIndex={at === ADD ? 0 : -1} onFocus={() => setAt(ADD)} onClick={() => setSheet({ id: undefined })}>
              <Plus size={20} aria-hidden="true" />
            </button>
          </Tooltip>
        </div>
        <span className="deck-sr" role="status" aria-live="polite">
          {said}
        </span>
        {flight?.what === "not_read" ? (
          <p className="deck-flight-edit__said" role="status">
            Actions in {named} not read: {flight.why}
          </p>
        ) : null}
        {failed === undefined ? null : (
          <p className="deck-flight-edit__said" role="alert">
            {failed}
          </p>
        )}
      </FlightBlock>
      {sheet === undefined ? null : createPortal(adding, document.body)}
    </>
  );
}
