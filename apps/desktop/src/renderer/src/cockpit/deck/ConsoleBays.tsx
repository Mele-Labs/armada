// The bridge take's modular console (mock): five bays, each a socket that takes any instrument, and the rack that seats them. What is in
// each bay is kept in localStorage. Drawn only where `data-console="bridge"` is set (`console-bridge.css`); the markup is the same either way.

import { useEffect, useLayoutEffect, useRef, useState, useSyncExternalStore, type DragEvent, type KeyboardEvent, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { Activity as ActivityGlyph, ChartNoAxesGantt, Gauge, Maximize2, Megaphone, PlugZap, Plus, Radar, ShieldCheck, X, type LucideIcon } from "lucide-react";
import { Button, Kbd, Tooltip } from "@armada/components";

import { useZoomBox } from "../../job-panes/zoom";
import { glyphOf } from "./flight-glyphs";
import { ConsoleStage, type BayState, type Dot, type Kind, type Layout, type Plan } from "./pixi/console-stage";
import { tileCap } from "./session-tiles";
import type { tilesOf } from "./flight-controls";

export type InstrumentId = "matrix" | "beacon" | "radar" | "trace" | "scope" | "needles" | "usage";
type Tile = ReturnType<typeof tilesOf>[number];

/** The code is a stencil on the console, decoration only. `mod` names the mod an instrument comes from. */
export const INSTRUMENTS: Record<InstrumentId, { code: string; label: string; Icon: LucideIcon; mod?: string }> = {
  matrix: { code: "CHK", label: "Checks", Icon: ShieldCheck },
  beacon: { code: "ALM", label: "Calls", Icon: Megaphone },
  radar: { code: "RDR", label: "Map", Icon: Radar },
  trace: { code: "TLM", label: "Timeline", Icon: ChartNoAxesGantt },
  scope: { code: "ACT", label: "Activity", Icon: ActivityGlyph },
  needles: { code: "GGE", label: "Gauges", Icon: Gauge },
  usage: { code: "USE", label: "Plan usage", Icon: PlugZap, mod: "plan-usage" },
};
const BUILT_IN: readonly InstrumentId[] = ["matrix", "beacon", "radar", "trace", "scope", "needles"];
const FROM_MODS: readonly InstrumentId[] = ["usage"];
/** The deck tile each key instrument stands for, which holds its words, glyph and key. */
const TILE_OF: Partial<Record<InstrumentId, string>> = { matrix: "checks", beacon: "needs", radar: "map", trace: "timeline" };
const TIP_OF: Partial<Record<string, string>> = { checks: "Checks: only what is running one" };

// ---- What the bays hold ------------------------------------------------------------------------------

/** Which take of the console is drawn: the flat one, or on the canvas with the Map or Dispatch at its centre (`&centre=`). Each keeps its own bays. */
type Take = "flat" | "map" | "dispatch";
const takeOf = (): Take => {
  const root = typeof document === "undefined" ? undefined : document.documentElement;
  if (root === undefined || !root.hasAttribute("data-console-pixi")) return "flat";
  return root.dataset.consoleCentre === "dispatch" ? "dispatch" : "map";
};
type Bays = readonly (InstrumentId | null)[];
/** The flanking bays of each take. The centrepiece is not one of them. */
const FIRST: Record<Take, Bays> = {
  flat: ["matrix", "beacon", "radar", "trace", "scope"],
  map: ["matrix", "beacon", "trace", "scope"],
  dispatch: ["matrix", "beacon", "radar", "trace", "scope", "needles"],
};
const heard = new Set<() => void>();
const kept = (take: Take) => (take === "flat" ? "armada.console.bays" : `armada.console.bays.${take}`);

const read = (take: Take): Bays => {
  try {
    const found: unknown = JSON.parse(window.localStorage.getItem(kept(take)) ?? "null");
    if (Array.isArray(found) && found.length === FIRST[take].length && found.every((one) => one === null || (typeof one === "string" && one in INSTRUMENTS))) return found as Bays;
  } catch {
    /* a bad record is the default */
  }
  return FIRST[take];
};
const held: Partial<Record<Take, Bays>> = {};
const baysOf = (take: Take): Bays => (held[take] ??= read(take));

/** Put an instrument in a bay, or empty it. */
function seat(bay: number, id: InstrumentId | null): void {
  const take = takeOf();
  held[take] = baysOf(take).map((one, at) => (at === bay ? id : one));
  try {
    window.localStorage.setItem(kept(take), JSON.stringify(held[take]));
  } catch {
    /* kept for this page only */
  }
  heard.forEach((one) => one());
}
const useBays = (): Bays =>
  useSyncExternalStore(
    (hear) => (heard.add(hear), () => void heard.delete(hear)),
    () => baysOf(takeOf()),
  );

export type Wiring = {
  tiles: readonly Tile[];
  pressed: (tile: string) => boolean;
  press: (tile: string) => void;
  /** A Job is asking him. */
  calling: boolean;
  /** The Activity graph, drawn into the scope. */
  scope: ReactNode;
  /** The Dispatch key, which is the last facet of the arc where the console is drawn on a canvas. */
  dispatch?: ReactNode;
};

/** Whether the console is drawn on a canvas (`?console=bridge`, unless `?pixi=0`). */
const onCanvas = () => typeof document !== "undefined" && document.documentElement.hasAttribute("data-console-pixi");

// ---- The bays ------------------------------------------------------------------------------------------

/** A key instrument is the deck's own tile, drawn as an instrument: it does what the tile does. */
function Key({ id, wiring }: { id: InstrumentId; wiring: Wiring }) {
  const tileId = TILE_OF[id]!;
  const tile = wiring.tiles.find((one) => one.id === tileId);
  const Glyph = glyphOf(tile?.glyph, "zap");
  const label = tile?.label ?? INSTRUMENTS[id].label;
  return (
    <Tooltip label={TIP_OF[tileId] ?? label} {...(tile?.key === undefined ? {} : { shortcut: tile.key })} asChild>
      <button
        type="button"
        className="deck-tile"
        aria-label={label}
        data-key={tile?.key}
        data-tile={tileId}
        data-calling={tileId === "needs" && wiring.calling ? "" : undefined}
        aria-pressed={wiring.pressed(tileId)}
        onClick={() => wiring.press(tileId)}
      >
        <i className="console-a" aria-hidden="true" />
        <i className="console-b" aria-hidden="true" />
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

/** The Activity graph as a scope. Its press opens Activity over the Cockpit's body. */
function Scope({ wiring }: { wiring: Wiring }) {
  const [open, setOpen] = useState(false);
  const box = useZoomBox(open, 0);
  useEffect(() => {
    if (!open) return;
    const close = (event: globalThis.KeyboardEvent) => event.key === "Escape" && setOpen(false);
    document.addEventListener("keydown", close);
    return () => document.removeEventListener("keydown", close);
  }, [open]);
  return (
    <div className="console-scope" data-tile="scope">
      {wiring.scope}
      <span className="console-scope__open">
        <Tooltip label="Activity" asChild>
          <Button variant="ghost" size="sm" iconOnly aria-label="Activity" aria-expanded={open} onClick={() => setOpen(true)}>
            <Maximize2 size={12} aria-hidden="true" />
          </Button>
        </Tooltip>
      </span>
      {open && box !== undefined
        ? createPortal(
            <div className="console-sheet" role="dialog" aria-label="Activity" style={{ left: box.left, top: box.top, width: box.width, height: box.height }}>
              <header className="console-sheet__band">
                <Tooltip label="Close" shortcut="Esc" asChild>
                  <Button variant="ghost" size="sm" iconOnly aria-label="Close" autoFocus onClick={() => setOpen(false)}>
                    <X size={16} aria-hidden="true" />
                  </Button>
                </Tooltip>
              </header>
              <div className="console-sheet__body">{wiring.scope}</div>
            </div>,
            document.body,
          )
        : null}
    </div>
  );
}

function Seated({ id, wiring }: { id: InstrumentId; wiring: Wiring }) {
  if (id === "scope") return <Scope wiring={wiring} />;
  if (id === "needles") {
    return (
      <Tooltip label={INSTRUMENTS.needles.label} asChild>
        <div className="console-gauges" role="img" aria-label={INSTRUMENTS.needles.label}>
          <i />
          <i />
        </div>
      </Tooltip>
    );
  }
  if (id === "usage") {
    return (
      <Tooltip label={INSTRUMENTS.usage.label} asChild>
        <div className="console-usage" role="img" aria-label={INSTRUMENTS.usage.label}>
          <span className="console-usage__mod" aria-hidden="true">
            {INSTRUMENTS.usage.mod}
          </span>
          <i style={{ ["--v" as string]: "42%" }} />
          <i style={{ ["--v" as string]: "71%" }} />
        </div>
      </Tooltip>
    );
  }
  return <Key id={id} wiring={wiring} />;
}

const DOTS_EVERY = 400;

/** The console drawn on a canvas: the canvas is the picture, and over each projected facet stands the instrument's own button, unseen, which keeps its press, label, tooltip and focus. */
function CanvasBays({ wiring }: { wiring: Wiring }) {
  const seated = useBays();
  const host = useRef<HTMLDivElement>(null);
  const stage = useRef<ConsoleStage | undefined>(undefined);
  const els = useRef<(HTMLDivElement | null)[]>([]);
  const [layout, setLayout] = useState<Layout | undefined>();
  const take = takeOf();
  const dispatch = wiring.dispatch === undefined ? null : "dispatch";
  const lit = document.documentElement.hasAttribute("data-console-lit");
  // The centrepiece is fixed per take and the seated instruments flank it: the Map with Dispatch keyed on its lip, or Dispatch with the Map beside it.
  const [plan, items]: [Plan, (InstrumentId | "dispatch" | null)[]] =
    take === "dispatch"
      ? [{ widths: [1, 1, 1, 1.9, 1, 1, 1], depths: [1, 1, 1, 1.9, 1, 1, 1], keyed: undefined, lit }, [seated[0]!, seated[1]!, seated[2]!, dispatch, seated[3]!, seated[4]!, seated[5]!]]
      : [{ widths: [1, 1, 1.9, 1, 1], depths: [1, 1, 1.9, 1, 1], keyed: 2, lit }, [seated[0]!, seated[1]!, "radar", seated[2]!, seated[3]!, dispatch]];
  const dots = useRef<{ at: number; list: Dot[] }>({ at: 0, list: [] });
  useLayoutEffect(() => {
    const canvas = host.current!;
    const block = canvas.parentElement!;
    const made = ConsoleStage.start(canvas, plan);
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
    return () => {
      watch.disconnect();
      block.removeEventListener("pointerenter", poke, true);
      block.removeEventListener("pointerleave", poke, true);
      block.removeEventListener("focusin", poke);
      block.removeEventListener("focusout", poke);
      block.style.height = "";
      made.destroy();
      stage.current = undefined;
    };
  }, []);
  // What each frame draws is read from the buttons, so a press, a hover or a call changes the picture with no state of its own.
  useLayoutEffect(() => {
    stage.current?.setSource((): BayState[] =>
      items.map((id, at) => {
        const el = els.current[at];
        if (id === null || el === null || el === undefined) return { kind: null, pressed: false, calling: false, lit: false, focused: false };
        const kind: Kind = id === "dispatch" ? "dispatch" : id;
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
                      return { x: (box.left + box.width / 2 - plot.left) / plot.width, y: (box.top + box.height / 2 - plot.top) / plot.height, r: Math.max(1.6, box.width / 2), colour: getComputedStyle(dot).getPropertyValue("--tone") || "#888" };
                    }),
            };
          }
          list = dots.current.list;
        }
        return { kind, pressed: el.querySelector("[aria-pressed='true']") !== null, calling: el.querySelector("[data-calling]") !== null, lit: el.matches(":hover, :focus-within"), focused: el.querySelector(".deck-tile:focus-visible") !== null, ...(list === undefined ? {} : { dots: list }) };
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
                {id === "dispatch" ? "Dispatch" : INSTRUMENTS[id].label}
              </span>
            )}
          </div>
        );
      })}
    </>
  );
}

/** The console's row of bays. A bay with nothing in it is a dark, recessed socket. */
export function ConsoleBays({ wiring }: { wiring: Wiring }) {
  return onCanvas() ? <CanvasBays wiring={wiring} /> : <FlatBays wiring={wiring} />;
}

function FlatBays({ wiring }: { wiring: Wiring }) {
  const seated = useBays();
  return (
    <>
      {seated.map((id, at) => (
        <div key={at} className="console-bay" data-n={at + 1} data-empty={id === null || undefined}>
          {id === null ? null : <Seated id={id} wiring={wiring} />}
        </div>
      ))}
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
export function ConsoleRack({ host, onDone }: { host: HTMLElement | null; onDone: () => void }) {
  const seated = useBays();
  const [armed, setArmed] = useState<number | undefined>();
  const [over, setOver] = useState<number | undefined>();
  const last = useRef(0);
  const sockets = useRef<(HTMLButtonElement | null)[]>([]);
  const tray = useRef<HTMLDivElement>(null);
  const put = (bay: number, id: InstrumentId | null) => {
    seat(bay, id);
    setArmed(undefined);
    sockets.current[bay]?.focus();
  };
  const drop = (bay: number) => (event: DragEvent<HTMLElement>) => {
    event.preventDefault();
    setOver(undefined);
    const id = event.dataTransfer.getData("text/plain");
    if (id in INSTRUMENTS) put(bay, id as InstrumentId);
  };
  useEffect(() => {
    if (armed !== undefined) tray.current?.querySelector<HTMLElement>("button")?.focus();
  }, [armed]);
  const tile = (id: InstrumentId) => {
    const { Icon, code, label, mod } = INSTRUMENTS[id];
    return (
      <Tooltip key={id} label={mod === undefined ? label : `${label}, ${mod}`} asChild>
        <button
          type="button"
          role="option"
          aria-selected={false}
          aria-label={label}
          className="console-tray__item"
          data-mod={mod}
          draggable
          onDragStart={(event) => event.dataTransfer.setData("text/plain", id)}
          onClick={() => put(armed ?? last.current, id)}
        >
          <Icon size={16} aria-hidden="true" />
          <span className="console-tray__code" aria-hidden="true">
            {code}
          </span>
          {mod === undefined ? null : (
            <span className="console-tray__mod" aria-hidden="true">
              {mod}
            </span>
          )}
        </button>
      </Tooltip>
    );
  };
  return (
    <>
      {host === null
        ? null
        : createPortal(
            <>
              <Tooltip label="Leave without saving" shortcut="Esc">
                <Button variant="ghost" size="sm" onClick={onDone}>
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
        className="deck-controls__block console-rack"
        data-flight-editing=""
        onKeyDown={(event) => {
          if (event.key === "Escape" && armed !== undefined) {
            event.stopPropagation();
            setArmed(undefined);
            sockets.current[armed]?.focus();
          }
        }}
      >
        <div className="console-rack__bays" onKeyDown={(event) => along(event, ".console-socket")}>
          {seated.map((id, at) => {
            const meta = id === null ? undefined : INSTRUMENTS[id];
            const Icon = meta?.Icon;
            return (
              <Tooltip key={at} label={meta?.label ?? `Bay ${at + 1}`} asChild>
                <button
                  type="button"
                  className="console-socket"
                  data-n={at + 1}
                  data-empty={id === null || undefined}
                  data-armed={armed === at || undefined}
                  data-over={over === at || undefined}
                  aria-label={`Bay ${at + 1}${meta === undefined ? "" : `, ${meta.label}`}`}
                  aria-pressed={armed === at}
                  ref={(element) => void (sockets.current[at] = element)}
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
                  {meta === undefined ? null : (
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
          {BUILT_IN.map(tile)}
          <span className="console-tray__rule" aria-hidden="true" />
          {FROM_MODS.map(tile)}
          <Tooltip label="Make one" asChild>
            <Button variant="ghost" size="sm" className="console-tray__make">
              <Plus size={14} aria-hidden="true" />
              Make one
            </Button>
          </Tooltip>
        </div>
      </div>
    </>
  );
}
