// The console: four bays round the Map, each a socket that takes any instrument, and the rack that seats them. The canvas is the picture
// (`pixi/console-stage.ts`); over each projected facet stands the instrument's own button, unseen, which keeps its press, label, tooltip and focus.

import { useEffect, useLayoutEffect, useRef, useState, useSyncExternalStore, type DragEvent, type KeyboardEvent, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { Activity as ActivityGlyph, ChartNoAxesGantt, Gauge, Megaphone, Plus, Radar, ShieldCheck, type LucideIcon } from "lucide-react";
import { Button, Kbd, Tooltip } from "@armada/components";

import { ActivityHud } from "./ConsoleHud";
import { toggleCalls, useCallsHud } from "./ConsoleCalls";
import { FlightActionSheet } from "./FlightActionSheet";
import { entriesOf, isWebAddress, saveFlight, tilesOf, useFlightFile } from "./flight-controls";
import { glyphOf } from "./flight-glyphs";
import { ConsoleStage, type BayState, type Dot, type Kind, type Layout, type Plan } from "./pixi/console-stage";
import { tileCap } from "./session-tiles";

export type InstrumentId = "matrix" | "beacon" | "radar" | "trace" | "scope" | "needles";
/** What a bay holds: one of Armada's instruments, or a Page tile of the owner's, by its id in the file. */
export type Seat = InstrumentId | `page:${string}`;
type Tile = ReturnType<typeof tilesOf>[number];

/** The code is a stencil on the console, decoration only. */
export const INSTRUMENTS: Record<InstrumentId, { code: string; label: string; Icon: LucideIcon }> = {
  matrix: { code: "CHK", label: "Checks", Icon: ShieldCheck },
  beacon: { code: "ALM", label: "Calls", Icon: Megaphone },
  radar: { code: "RDR", label: "Map", Icon: Radar },
  trace: { code: "TLM", label: "Timeline", Icon: ChartNoAxesGantt },
  scope: { code: "ACT", label: "Activity", Icon: ActivityGlyph },
  needles: { code: "GGE", label: "Gauges", Icon: Gauge },
};
const IN_THE_TRAY = Object.keys(INSTRUMENTS) as InstrumentId[];
/** The deck tile each key instrument stands for, which holds its words, glyph and key. */
const TILE_OF: Partial<Record<InstrumentId, string>> = { matrix: "checks", beacon: "needs", radar: "map", trace: "timeline" };
const TIP_OF: Partial<Record<string, string>> = { checks: "Checks: only what is running one" };

const PAGE = "page:";
/** A Page tile's stencil, as the instruments have one: the first letters of its label. Decoration only. */
const stencilOf = (label: string): string => label.replace(/[^a-z0-9]/gi, "").slice(0, 3).toUpperCase();
const isSeat = (one: unknown): one is Seat => typeof one === "string" && (one in INSTRUMENTS || one.startsWith(PAGE));
/** The Page tiles the owner added to the deck, which the tray offers. */
const pagesOf = (tiles: readonly Tile[]): Tile[] => tiles.filter((one) => one.builtIn === undefined && one.kind === "page");
/** The Page tile a seat stands for, or nothing for an instrument, and for a tile since deleted. */
const pageOf = (seat: Seat | "dispatch" | null, tiles: readonly Tile[]): Tile | undefined => (typeof seat === "string" && seat.startsWith(PAGE) ? pagesOf(tiles).find((one) => one.id === seat.slice(PAGE.length)) : undefined);

// ---- What the bays hold ------------------------------------------------------------------------------

type Bays = readonly (Seat | null)[];
/** The bays either side of the Map, which is not one of them. */
const FIRST: Bays = ["matrix", "beacon", "trace", "scope"];
/** A viewer's convenience kept as a fold is (`collapse.ts`): `flight-controls.toml` is the wire's, and Fleet refuses a key it does not know. */
const KEPT = "armada.console.bays";
const heard = new Set<() => void>();

const read = (): Bays => {
  try {
    const found: unknown = JSON.parse(window.localStorage.getItem(KEPT) ?? "null");
    if (Array.isArray(found) && found.length === FIRST.length && found.every((one) => one === null || isSeat(one))) return found as Bays;
  } catch {
    /* a bad record is the default */
  }
  return FIRST;
};
let held: Bays | undefined;
const baysOf = (): Bays => (held ??= read());
/** The next read is the storage's again, as a test that cleared the storage needs. */
export const forgetBays = (): void => {
  held = undefined;
};

function keepBays(bays: Bays): void {
  held = bays;
  try {
    window.localStorage.setItem(KEPT, JSON.stringify(bays));
  } catch {
    /* kept for this page only */
  }
  heard.forEach((one) => one());
}
/** Put an instrument in a bay, or empty it. */
const seat = (bay: number, id: Seat | null): void => keepBays(baysOf().map((one, at) => (at === bay ? id : one)));
const useBays = (): Bays =>
  useSyncExternalStore(
    (hear) => (heard.add(hear), () => void heard.delete(hear)),
    baysOf,
  );

export type Wiring = {
  tiles: readonly Tile[];
  pressed: (tile: string) => boolean;
  press: (tile: string) => void;
  /** A Job is asking him. */
  calling: boolean;
  /** The Activity graph, drawn into the scope, and at the HUD's size; absent where a layout hides Activity. */
  scope?: { small: ReactNode; large: ReactNode };
  /** Opens a page, as Bridge opens an external one. */
  open: (address: string) => void;
  /** The Dispatch key on the Map's lip; absent with a Job open or where a layout hides Dispatch. */
  dispatch?: ReactNode;
  /** The panel is folded away: the canvas holds its frame. */
  folded: boolean;
};

// ---- The bays ------------------------------------------------------------------------------------------

/** A key instrument is the deck's own tile: it does what the tile does. */
function Key({ id, wiring }: { id: InstrumentId; wiring: Wiring }) {
  const tileId = TILE_OF[id]!;
  const tile = wiring.tiles.find((one) => one.id === tileId);
  const Glyph = glyphOf(tile?.glyph, "zap");
  const label = tile?.label ?? INSTRUMENTS[id].label;
  // The Calls facet opens the Calls HUD.
  const calls = tileId === "needs";
  const hud = useCallsHud();
  return (
    <Tooltip label={TIP_OF[tileId] ?? label} {...(tile?.key === undefined ? {} : { shortcut: tile.key })} asChild>
      <button
        type="button"
        className="deck-tile"
        aria-label={label}
        data-key={tile?.key}
        data-tile={tileId}
        data-calling={calls && wiring.calling ? "" : undefined}
        aria-pressed={calls ? hud.open : wiring.pressed(tileId)}
        onClick={() => (calls ? toggleCalls() : wiring.press(tileId))}
      >
        <Glyph size={20} aria-hidden="true" />
        <span className="deck-tile__text">{tile?.text ?? label}</span>
        {tile?.key === undefined ? null : (
          <Kbd className="deck-tile__key" aria-hidden="true">
            {tileCap(tile.key)}
          </Kbd>
        )}
      </button>
    </Tooltip>
  );
}

/** A Page tile of the owner's seated in a bay: a key that opens its address. */
function PageKey({ tile, wiring }: { tile: Tile; wiring: Wiring }) {
  const Glyph = glyphOf(tile.glyph, "zap");
  const address = (tile.words ?? "").trim();
  return (
    <Tooltip label={`Open ${address}`} {...(tile.key === undefined ? {} : { shortcut: tile.key })} asChild>
      <button type="button" className="deck-tile" aria-label={tile.label} data-key={tile.key} data-tile={tile.id} onClick={() => isWebAddress(address) && wiring.open(address)}>
        <Glyph size={20} aria-hidden="true" />
        <span className="deck-tile__text">{tile.text}</span>
        {tile.key === undefined ? null : (
          <Kbd className="deck-tile__key" aria-hidden="true">
            {tileCap(tile.key)}
          </Kbd>
        )}
      </button>
    </Tooltip>
  );
}

/** The Activity graph as a scope. The whole facet is one press, which opens the HUD over the field; its graph only feeds the picture, so its dots take neither pointer nor focus. */
function Scope({ graph }: { graph: NonNullable<Wiring["scope"]> }) {
  const [open, setOpen] = useState(false);
  const facet = useRef<HTMLDivElement>(null);
  const close = () => {
    setOpen(false);
    facet.current?.querySelector<HTMLElement>(".console-scope__press")?.focus();
  };
  return (
    <div ref={facet} className="console-scope" data-tile="scope">
      <div className="console-scope__graph" inert>
        {graph.small}
      </div>
      <Tooltip label="Activity" asChild>
        <button type="button" className="console-scope__press" aria-label="Activity" aria-pressed={open} aria-haspopup="dialog" onClick={() => (open ? close() : setOpen(true))} />
      </Tooltip>
      {open ? <ActivityHud graph={graph.large} facet={facet} onClose={close} /> : null}
    </div>
  );
}

function Seated({ id, wiring }: { id: Seat; wiring: Wiring }) {
  const page = pageOf(id, wiring.tiles);
  if (page !== undefined) return <PageKey tile={page} wiring={wiring} />;
  if (!(id in INSTRUMENTS)) return null;
  const instrument = id as InstrumentId;
  if (instrument === "scope") return wiring.scope === undefined ? null : <Scope graph={wiring.scope} />;
  if (instrument === "needles") {
    return (
      <Tooltip label={INSTRUMENTS.needles.label} asChild>
        <div className="console-gauges" role="img" aria-label={INSTRUMENTS.needles.label} />
      </Tooltip>
    );
  }
  return <Key id={instrument} wiring={wiring} />;
}

const DOTS_EVERY = 400;
/** The Map is the wide facet at the middle, with Dispatch keyed on its lip. */
const PLAN: Plan = { widths: [1, 1, 1.9, 1, 1], depths: [1, 1, 1.9, 1, 1], keyed: 2 };

/** The console's row of bays. A bay with nothing in it is a dark, recessed socket. */
export function ConsoleBays({ wiring }: { wiring: Wiring }) {
  const seated = useBays();
  const host = useRef<HTMLDivElement>(null);
  const stage = useRef<ConsoleStage | undefined>(undefined);
  const els = useRef<(HTMLDivElement | null)[]>([]);
  const [layout, setLayout] = useState<Layout | undefined>();
  const dispatch = wiring.dispatch === undefined ? null : "dispatch";
  // A bay holding a Page tile since deleted is an empty socket.
  const shown = (id: Seat | null) => (id === "scope" && wiring.scope === undefined ? null : id !== null && id.startsWith(PAGE) && pageOf(id, wiring.tiles) === undefined ? null : id);
  const items: (Seat | "dispatch" | null)[] = [shown(seated[0]!), shown(seated[1]!), "radar", shown(seated[2]!), shown(seated[3]!), dispatch];
  const dots = useRef<{ at: number; list: Dot[] }>({ at: 0, list: [] });
  useLayoutEffect(() => {
    const canvas = host.current!;
    const block = canvas.parentElement!;
    // The buttons are placed from the layout whether or not a canvas ever comes; without one they have to be seen.
    const made = ConsoleStage.start(canvas, PLAN, () => block.setAttribute("data-no-canvas", ""));
    stage.current = made;
    const measure = () => {
      const next = made.resize(block.clientWidth);
      canvas.style.height = `${next.height}px`;
      block.style.height = `${next.height}px`;
      setLayout((was) => (was !== undefined && was.width === next.width && was.height === next.height ? was : next));
    };
    measure();
    const watch = new ResizeObserver(measure);
    watch.observe(block);
    const poke = () => made.poke();
    block.addEventListener("pointerenter", poke, true);
    block.addEventListener("pointerleave", poke, true);
    block.addEventListener("focusin", poke);
    block.addEventListener("focusout", poke);
    // A HUD opening presses a facet with no render of this component, and with no frame coming the canvas has to hear of it.
    const marks = new MutationObserver(poke);
    marks.observe(block, { subtree: true, attributes: true, attributeFilter: ["aria-pressed", "data-calling"] });
    return () => {
      watch.disconnect();
      marks.disconnect();
      block.removeEventListener("pointerenter", poke, true);
      block.removeEventListener("pointerleave", poke, true);
      block.removeEventListener("focusin", poke);
      block.removeEventListener("focusout", poke);
      block.style.height = "";
      block.removeAttribute("data-no-canvas");
      made.destroy();
      stage.current = undefined;
    };
  }, []);
  useLayoutEffect(() => stage.current?.setHidden(wiring.folded), [wiring.folded]);
  // What each frame draws is read from the buttons, so a press, a hover or a call changes the picture with no state of its own.
  useLayoutEffect(() => {
    stage.current?.setSource((): BayState[] =>
      items.map((id, at) => {
        const el = els.current[at];
        if (id === null || el === null || el === undefined) return { kind: null, pressed: false, calling: false, lit: false, focused: false };
        const page = pageOf(id, wiring.tiles);
        const kind: Kind = id === "dispatch" ? "dispatch" : page === undefined ? (id as InstrumentId) : "key";
        let list: Dot[] | undefined;
        if (id === "scope") {
          const now = performance.now();
          if (now - dots.current.at > DOTS_EVERY) {
            const plot = el.querySelector<HTMLElement>(".armada-tl__plot")?.getBoundingClientRect();
            dots.current = {
              at: now,
              list:
                plot === undefined || plot.width === 0
                  ? []
                  : [...el.querySelectorAll<HTMLElement>(".armada-tl__dot")].map((dot) => {
                      const box = dot.getBoundingClientRect();
                      const style = getComputedStyle(dot);
                      return { x: (box.left + box.width / 2 - plot.left) / plot.width, y: (box.top + box.height / 2 - plot.top) / plot.height, r: Math.max(1.6, box.width / 2), colour: style.getPropertyValue("--tone") || style.color };
                    }),
            };
          }
          list = dots.current.list;
        }
        return { kind, pressed: el.querySelector("[aria-pressed='true']") !== null, calling: el.querySelector("[data-calling]") !== null, lit: el.matches(":hover, :focus-within"), focused: el.querySelector(":is(.deck-tile, .console-scope__press):focus-visible") !== null, ...(list === undefined ? {} : { dots: list }), ...(page === undefined ? {} : { glyph: page.glyph }) };
      }),
    );
    stage.current?.poke();
  });
  return (
    <>
      <div ref={host} className="console-pixi" />
      {items.map((id, at) => {
        const room = layout?.inner[at];
        const label = layout?.labels[at];
        return (
          <div
            key={at}
            ref={(element) => void (els.current[at] = element)}
            className="console-bay"
            data-n={at + 1}
            data-empty={id === null || undefined}
            style={room === undefined ? { display: "none" } : { left: room.x, top: room.y, width: room.w, height: room.h }}
          >
            {id === null ? null : id === "dispatch" ? wiring.dispatch : <Seated id={id} wiring={wiring} />}
            {id === null || room === undefined || label === undefined ? null : (
              // The word is a straight piece of the page, standing on the lip; the bay's button runs under it and takes the press.
              <span
                className="console-bay__label"
                aria-hidden="true"
                style={{ left: label.x - room.x, top: label.y - room.y, fontSize: label.size, transform: `translate(-50%, -50%) rotate(${label.angle}rad)` }}
              >
                {id === "dispatch" ? "Dispatch" : (pageOf(id, wiring.tiles)?.label ?? INSTRUMENTS[id as InstrumentId].label)}
              </span>
            )}
          </div>
        );
      })}
    </>
  );
}

// ---- The rack: every bay an open socket, and a tray of instruments to put in them ---------------------------

/** Left and Right move between the buttons of the row the key was pressed in. */
function along(event: KeyboardEvent<HTMLElement>, selector: string) {
  if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
  const row = [...event.currentTarget.querySelectorAll<HTMLElement>(selector)];
  const at = row.indexOf(document.activeElement as HTMLElement);
  if (at === -1) return;
  event.preventDefault();
  row[Math.min(row.length - 1, Math.max(0, at + (event.key === "ArrowLeft" ? -1 : 1)))]?.focus();
}

/** The rack. Focus a bay and press Enter to pick it, then choose from the tray with the arrows and Enter; or drag an instrument onto a bay. Delete empties the bay. */
export function ConsoleRack({ host, tiles, onDone }: { host: HTMLElement | null; tiles: readonly Tile[]; onDone: () => void }) {
  const seated = useBays();
  const file = useFlightFile();
  const [adding, setAdding] = useState(false);
  const pages = pagesOf(tiles);
  const [armed, setArmed] = useState<number | undefined>();
  const [over, setOver] = useState<number | undefined>();
  const last = useRef(0);
  const rack = useRef<HTMLDivElement>(null);
  // Found in the page: `Tooltip` takes the ref of the button it wraps.
  const focusSocket = (bay: number) => rack.current?.querySelectorAll<HTMLElement>(".console-socket")[bay]?.focus();
  const tray = useRef<HTMLDivElement>(null);
  const before = useRef(seated);
  const cancel = () => {
    keepBays(before.current);
    onDone();
  };
  const put = (bay: number, id: Seat | null) => {
    seat(bay, id);
    setArmed(undefined);
    focusSocket(bay);
  };
  const drop = (bay: number) => (event: DragEvent<HTMLElement>) => {
    event.preventDefault();
    setOver(undefined);
    const id = event.dataTransfer.getData("text/plain");
    if (id in INSTRUMENTS || pageOf(id as Seat, tiles) !== undefined) put(bay, id as Seat);
  };
  useEffect(() => {
    if (armed !== undefined) tray.current?.querySelector<HTMLElement>("button")?.focus();
  }, [armed]);
  // Edit, which opened this, has gone from under the keys.
  useEffect(() => focusSocket(0), []);
  /** One thing the tray offers: an instrument by its stencil, or a Page tile by its own label and glyph. */
  const item = (id: Seat, label: string, Icon: LucideIcon, code?: string) => {
    return (
      <Tooltip key={id} label={label} asChild>
        <button
          type="button"
          role="option"
          aria-selected={false}
          aria-label={label}
          className="console-tray__item"
          draggable
          onDragStart={(event) => event.dataTransfer.setData("text/plain", id)}
          onClick={() => put(armed ?? last.current, id)}
        >
          <Icon size={16} aria-hidden="true" />
          {code === undefined ? null : (
            <span className="console-tray__code" aria-hidden="true">
              {code}
            </span>
          )}
        </button>
      </Tooltip>
    );
  };
  const all = tilesOf("deck", file);
  return (
    <>
      {host === null
        ? null
        : createPortal(
            <>
              <Tooltip label="Leave without saving" shortcut="Esc">
                <Button variant="ghost" size="sm" onClick={cancel}>
                  Cancel
                </Button>
              </Tooltip>
              <Button variant="secondary" size="sm" onClick={onDone}>
                Done
              </Button>
            </>,
            host,
          )}
      <div
        ref={rack}
        className="deck-controls__block console-rack"
        data-flight-editing=""
        onKeyDown={(event) => {
          if (event.key !== "Escape") return;
          event.stopPropagation();
          if (armed === undefined) return cancel();
          setArmed(undefined);
          focusSocket(armed);
        }}
      >
        <div className="console-rack__bays" onKeyDown={(event) => along(event, ".console-socket")}>
          {seated.map((id, at) => {
            const page = pageOf(id, tiles);
            // A Page tile since deleted is an empty socket here as it is on the console.
            const meta = id === null || (id.startsWith(PAGE) && page === undefined) ? undefined : page !== undefined ? { label: page.label, Icon: glyphOf(page.glyph, "zap"), code: stencilOf(page.label) } : INSTRUMENTS[id as InstrumentId];
            const Icon = meta?.Icon;
            return (
              <Tooltip key={at} label={meta?.label ?? `Bay ${at + 1}`} asChild>
                <button
                  type="button"
                  className="console-socket"
                  data-n={at + 1}
                  data-empty={meta === undefined || undefined}
                  data-armed={armed === at || undefined}
                  data-over={over === at || undefined}
                  aria-label={`Bay ${at + 1}${meta === undefined ? "" : `, ${meta.label}`}`}
                  aria-pressed={armed === at}
                  onFocus={() => (last.current = at)}
                  onClick={() => setArmed(armed === at ? undefined : at)}
                  onKeyDown={(event) => {
                    if (event.key === "Delete" || event.key === "Backspace") {
                      event.preventDefault();
                      seat(at, null);
                    }
                  }}
                  onDragOver={(event) => (event.preventDefault(), setOver(at))}
                  onDragLeave={() => setOver(undefined)}
                  onDrop={drop(at)}
                >
                  {Icon === undefined ? null : <Icon size={20} aria-hidden="true" />}
                  {meta?.code === undefined ? null : (
                    <span className="console-socket__code" aria-hidden="true">
                      {meta.code}
                    </span>
                  )}
                </button>
              </Tooltip>
            );
          })}
        </div>
        <div className="console-tray" ref={tray} role="listbox" aria-label="Instruments" onKeyDown={(event) => along(event, ".console-tray__item")}>
          {IN_THE_TRAY.map((id) => item(id, INSTRUMENTS[id].label, INSTRUMENTS[id].Icon, INSTRUMENTS[id].code))}
          {pages.map((one) => item(`${PAGE}${one.id}`, one.label, glyphOf(one.glyph, "zap"), stencilOf(one.label)))}
          <Tooltip label="Add an action" asChild>
            <button type="button" role="option" aria-selected={false} aria-label="Add action" className="console-tray__item" onClick={() => setAdding(true)}>
              <Plus size={16} aria-hidden="true" />
            </button>
          </Tooltip>
        </div>
      </div>
      {/* Over the whole window and outside the rack, whose keys the sheet's Esc is not. */}
      {adding ? (
        createPortal(
          <FlightActionSheet
            set="deck"
            tile={undefined}
            tiles={all}
            entries={entriesOf(all)}
            onClose={() => setAdding(false)}
            onSave={(entry) => {
              void saveFlight("deck", [...entriesOf(all), entry]);
              setAdding(false);
            }}
          />,
          document.body,
        )
      ) : null}
    </>
  );
}
