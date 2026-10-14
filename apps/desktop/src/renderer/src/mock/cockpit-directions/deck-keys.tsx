// The flight deck's keys. One central handler lives in `Deck.tsx`; this file is what it reads: the panels and the key
// each answers to, moving focus between them and inside one, the two-key page jumps, and the sheet that lists it all.
// The cursor is DOM focus, as `list-keyboard` has it: a panel is lit by `:focus-within`, so the lit panel is where the
// keys act. Mock only. proto/cockpit-flight-deck.

import { useEffect, useRef } from "react";
import { Kbd, Sheet, Tooltip } from "@armada/components";

export type PanelId = "activity" | "list" | "map" | "bays" | "calls" | "queue" | "controls" | "autopilot";

type Panel = {
  id: PanelId;
  /** The key that goes to the panel. */
  key: string;
  name: string;
  /** Where the panel is drawn. */
  root: string;
  /** What the arrows move between inside it: none for the map, whose arrows pan. */
  items?: string;
  /** Where focus lands when there is no better place. */
  home?: string;
  /** Left out of the round Tab and F6 go: its keys act while focus is in it. */
  aside?: true;
};

/** In the order Tab goes round them. */
export const PANELS: readonly Panel[] = [
  { id: "list", key: "l", name: "Jobs and Sessions", root: ".deck-list", items: ".deck-row" },
  { id: "activity", key: "", name: "Activity", root: ".deck-activity", items: ".armada-tl__who", aside: true },
  { id: "map", key: "m", name: "the map", root: ".deck-stage > .deck-pane" },
  { id: "bays", key: "b", name: "Bays", root: ".deck-bays", items: ".deck-bay" },
  { id: "calls", key: "c", name: "Incoming calls", root: ".deck-incoming", items: ".deck-hail" },
  { id: "queue", key: "q", name: "Merge queue", root: ".deck-queue", items: "button" },
  { id: "controls", key: "f", name: "Flight control", root: ".deck-controls", items: ".deck-tile" },
  { id: "autopilot", key: "a", name: "Autopilot", root: ".deck-autopilot", items: "button", home: '[role="switch"]' },
];

const rootOf = (panel: Panel) => document.querySelector<HTMLElement>(panel.root) ?? undefined;
const present = () => PANELS.filter((one) => one.aside !== true && rootOf(one) !== undefined);

/** The panel a key press or a focus is in. */
export function panelAt(target: EventTarget | null): Panel | undefined {
  return target instanceof Element ? PANELS.find((one) => rootOf(one)?.contains(target)) : undefined;
}

const itemsOf = (panel: Panel, root: HTMLElement) => (panel.items === undefined ? [] : [...root.querySelectorAll<HTMLElement>(panel.items)].filter((one) => !one.hasAttribute("disabled")));

/** The last thing focused in each panel, so going back to a panel finds it as it was left. */
const last = new Map<PanelId, HTMLElement>();
export function remember(target: EventTarget | null): void {
  const panel = panelAt(target);
  const item = panel?.items === undefined ? null : (target as Element).closest<HTMLElement>(panel.items);
  if (panel !== undefined && item !== null) last.set(panel.id, item);
}

/** Puts focus in a panel: the list on its cursor, the rest on what was last focused in them, else their first item. */
export function focusPanel(id: PanelId, cursor?: string): void {
  const panel = PANELS.find((one) => one.id === id);
  const root = panel === undefined ? undefined : rootOf(panel);
  if (panel === undefined || root === undefined) return;
  const kept = last.get(id);
  const to =
    (id === "map" ? root.querySelector<HTMLElement>(".bf") : undefined) ??
    (panel.home === undefined ? undefined : root.querySelector<HTMLElement>(panel.home)) ??
    (id === "list" && cursor !== undefined ? root.querySelector<HTMLElement>(`[data-row="${CSS.escape(cursor)}"]`) : undefined) ??
    (kept !== undefined && root.contains(kept) ? kept : undefined) ??
    itemsOf(panel, root)[0] ??
    root;
  if (to === root) root.setAttribute("tabindex", "-1");
  to.focus();
}

/** The panel after this one, going round; the first (or last) when focus is in none. Panels that are not drawn are skipped. */
export function cycleFrom(id: PanelId | undefined, by: 1 | -1): PanelId {
  const all = present();
  const at = all.findIndex((one) => one.id === id);
  return all[at < 0 ? (by === 1 ? 0 : all.length - 1) : (at + by + all.length) % all.length]?.id ?? "list";
}

/** Moves focus to the next item, or the previous, in the order they are drawn: j and k, along a column or along a row alike. */
export function stepBy(panel: Panel, from: EventTarget | null, by: 1 | -1): void {
  const root = rootOf(panel);
  if (root === undefined || panel.items === undefined) return;
  const items = itemsOf(panel, root);
  const here = (from instanceof Element ? from.closest(panel.items) : null) ?? (document.activeElement instanceof Element ? document.activeElement.closest(panel.items) : null);
  const was = items.findIndex((one) => one === here);
  items[was < 0 ? (by === 1 ? 0 : items.length - 1) : Math.min(items.length - 1, Math.max(0, was + by))]?.focus();
}

const AIM: Record<string, [number, number]> = { ArrowRight: [1, 0], ArrowLeft: [-1, 0], ArrowDown: [0, 1], ArrowUp: [0, -1] };
const centre = (el: Element) => {
  const r = el.getBoundingClientRect();
  return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
};

/** Moves focus to the nearest item the arrow points at, by where they are drawn. A list answers up and down, a row left and right, a grid all four. */
export function stepIn(panel: Panel, from: EventTarget | null, key: string): void {
  const root = rootOf(panel);
  const aim = AIM[key];
  if (root === undefined || aim === undefined) return;
  const items = itemsOf(panel, root);
  const here = (from instanceof Element && panel.items !== undefined ? from.closest(panel.items) : null) ?? (document.activeElement instanceof Element && panel.items !== undefined ? document.activeElement.closest(panel.items) : null);
  const was = items.findIndex((one) => one === here);
  if (was < 0) return items[0]?.focus();
  const a = centre(items[was]!);
  let best: { item: HTMLElement; score: number } | undefined;
  for (const item of items) {
    if (item === items[was]) continue;
    const c = centre(item);
    const along = (c.x - a.x) * aim[0] + (c.y - a.y) * aim[1];
    const across = Math.abs((c.x - a.x) * aim[1] - (c.y - a.y) * aim[0]);
    if (along < 4 || across > along * 1.5 + 8) continue;
    const score = along + across * 2;
    if (best === undefined || score < best.score) best = { item, score };
  }
  best?.item.focus();
}

/**
 * The deck's window listener: `useListKeydown`'s, taken in the capture phase. It hears a press before a walk's own keys
 * (arrows, Enter, `a`) do, and the walk can see the press was taken.
 */
export function useDeckKeys(press: (event: KeyboardEvent) => void): void {
  const latest = useRef(press);
  useEffect(() => {
    latest.current = press;
  });
  useEffect(() => {
    const listen = (event: KeyboardEvent): void => {
      if (document.querySelector('[role="dialog"], [role="alertdialog"]') !== null) return;
      latest.current(event);
    };
    window.addEventListener("keydown", listen, true);
    return () => window.removeEventListener("keydown", listen, true);
  }, []);
}

/** After this key, the next one is the second of a pair: `g` then `j`. Anything else drops it, and so does a pause. */
export function armChord(second: Readonly<Record<string, () => void>>): void {
  let timer = 0;
  const off = () => {
    window.removeEventListener("keydown", next, true);
    window.clearTimeout(timer);
  };
  function next(event: KeyboardEvent): void {
    if (["Shift", "Meta", "Alt", "Control"].includes(event.key)) return;
    off();
    const run = second[event.key];
    if (run === undefined || event.metaKey || event.ctrlKey || event.altKey) return;
    // Taken before the page in front sees it: `j` after `g` is not `j`.
    event.preventDefault();
    event.stopPropagation();
    run();
  }
  window.addEventListener("keydown", next, true);
  timer = window.setTimeout(off, 1500);
}

/** The panel's key, quiet at the end of its heading, and its name and key in the tooltip. */
export function PanelKey({ id }: { id: PanelId }) {
  const panel = PANELS.find((one) => one.id === id)!;
  return (
    <Tooltip label={`Go to ${panel.name}`} shortcut={panel.key}>
      <Kbd>{panel.key}</Kbd>
    </Tooltip>
  );
}

// ---- The sheet -------------------------------------------------------------------------------------------

type Row = readonly [keys: readonly string[], does: string];
const GROUPS: readonly { head: string; rows: readonly Row[] }[] = [
  {
    head: "Go",
    rows: [
      [["l"], "Jobs and Sessions"],
      [["m"], "Map"],
      [["b"], "Bays"],
      [["c"], "Incoming calls"],
      [["q"], "Merge queue"],
      [["f"], "Flight control"],
      [["a"], "Autopilot"],
      [["Tab", "⇧Tab"], "Next, previous"],
    ],
  },
  {
    head: "Within",
    rows: [
      [["j", "k", "↑", "↓", "←", "→"], "Move"],
      [["Enter"], "Open"],
      [["Esc"], "Back"],
    ],
  },
  {
    head: "Controls",
    rows: [
      [["1–6"], "Tiles"],
      [["a", "Space"], "Autopilot"],
    ],
  },
  {
    head: "Calls",
    rows: [
      [["w"], "Next"],
      [["⇧W"], "Previous"],
    ],
  },
  {
    head: "Map",
    rows: [
      [["↑", "↓", "←", "→"], "Pan"],
      [["+", "-"], "Zoom"],
      [["0"], "Whole field, or a Job's whole run"],
      [["Home"], "Locate"],
      [["v"], "A Job's mission or canvas"],
      [["t"], "Timeline"],
    ],
  },
  {
    head: "Replay",
    rows: [
      [["Space"], "Play, pause"],
      [["←", "→"], "Step an event"],
      [["Home"], "Restart"],
      [["Esc", "⇧L"], "Live"],
    ],
  },
  {
    head: "Pages",
    rows: [
      [["g", "c"], "Cockpit"],
      [["g", "j"], "Jobs"],
      [["g", "s"], "Sessions"],
      [["g", "y"], "System"],
      [["n"], "Dispatch"],
      [["⌘K"], "Search"],
      [["⌘["], "Back"],
      [["?"], "Keys"],
    ],
  },
];

/** Every key the deck answers, over the deck. */
export function DeckKeySheet({ open, onClose }: { open: boolean; onClose: () => void }) {
  useEffect(() => {
    if (!open) return;
    // Focus leaves the panel it was in while the sheet is up; it comes back to it after.
    const was = document.activeElement instanceof HTMLElement ? document.activeElement : undefined;
    return () => {
      if (was?.isConnected === true) was.focus();
    };
  }, [open]);
  return (
    <Sheet open={open} title="Keys" kind="deck-keys" size="panel" floating closeBinding="Esc" onClose={onClose}>
      <div className="armada-keys deck-keys">
        {GROUPS.map((group) => (
          <section key={group.head} className="armada-keys__group" aria-label={group.head}>
            <h3 className="armada-keys__head">{group.head}</h3>
            {group.rows.map(([keys, does]) => (
              <dl key={does} className="armada-keys__row">
                <dt>
                  {keys.map((key) => (
                    <Kbd key={key}>{key}</Kbd>
                  ))}
                </dt>
                <dd>{does}</dd>
              </dl>
            ))}
          </section>
        ))}
      </div>
    </Sheet>
  );
}
