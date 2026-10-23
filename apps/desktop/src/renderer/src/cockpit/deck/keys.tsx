// The deck's keys on the page: moving focus between the panels and inside one, the two-key page jumps, and the sheet that
// lists them. The cursor is DOM focus, as `list-keyboard` has it: a panel is lit by `:focus-within`, so the lit panel is where
// the keys act. What each key is, and which act it belongs to, is the registry's (`keys-model.ts`); this file only reads the page.
// Ported from proto/cockpit-flight-deck. #2301.

import { useEffect, useRef } from "react";
import { Kbd, Sheet, Tooltip } from "@armada/components";

import { popupIsOpen } from "@armada/screens/src/list-keyboard";

import { PANELS, keySheetOf, panelKey, type Panel, type PanelId } from "./keys-model";

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

/** Moves focus to the next item, or the previous, in the order they are drawn: along a column or along a row alike. */
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
 * The deck's window listener: `useListKeydown`'s, taken in the capture phase, so the deck hears a press before a walk's own
 * keys do. **It stands down for a dialog, a layer or an open menu**, whose keys are its own while it is up.
 */
export function useDeckKeys(press: (event: KeyboardEvent) => void): void {
  const latest = useRef(press);
  useEffect(() => {
    latest.current = press;
  });
  useEffect(() => {
    const listen = (event: KeyboardEvent): void => {
      if (document.querySelector('[role="dialog"], [role="alertdialog"]') !== null) return;
      // A layer opened over a Session's thread: Esc closes it, and is not the deck going back.
      if (event.key === "Escape" && document.querySelector("[data-keeps-escape]") !== null) return;
      // A menu or list open over the deck: Esc closes that, wherever focus is, and is not the deck going back.
      if (event.key === "Escape" && popupIsOpen()) return;
      // A layer that owns a letter hears it where focus is inside it, and the deck leaves it alone: the Artifacts layer's V.
      if (event.target instanceof Element && event.target.closest("[data-owns-keys]")?.getAttribute("data-owns-keys")?.split(" ").includes(event.key.toLowerCase()) === true) return;
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
  const key = panelKey(id);
  const name = PANELS.find((one) => one.id === id)?.name ?? id;
  if (key === "") return null;
  return (
    <Tooltip label={`Go to ${name}`} shortcut={key}>
      <Kbd>{key}</Kbd>
    </Tooltip>
  );
}

/** Every key the deck answers, over the deck: drawn from the registry. */
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
        {keySheetOf().map((group) => (
          <section key={group.head} className="armada-keys__group" aria-label={group.head}>
            <h3 className="armada-keys__head">{group.head}</h3>
            {group.rows.map((row) => (
              <dl key={`${row.does}:${row.keys.join(" ")}`} className="armada-keys__row">
                <dt>
                  {row.keys.map((key) => (
                    <Kbd key={key}>{key}</Kbd>
                  ))}
                </dt>
                <dd>{row.does}</dd>
              </dl>
            ))}
          </section>
        ))}
      </div>
    </Sheet>
  );
}
