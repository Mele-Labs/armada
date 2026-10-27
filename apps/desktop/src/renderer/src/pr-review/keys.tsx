// The review surface's keys, so every act on it is reachable without the mouse. **They are acts of `actions.toml`**
// (`review_*`, scope `review`), so Settings → Keyboard shortcuts lists and rebinds them and this file asks the keymap
// what a press is. The letters that go to a part (`l`, `f`) are the deck's own for the same places, so the same key goes
// to the same place. The list motions (`move_focus`, `open`, `close`) are the registry's `list` rows, shared with the deck.
//
// Focus is the cursor, as the deck has it: three parts (the list, the reader, the foot tools), a key to each, arrows
// inside each, Esc backing out a level. A key stands down in a text field and while a dialog is up.

import { useEffect } from "react";
import { isPressed, pressedSlot } from "@armada/components";
import { holdsText } from "@armada/screens/src/keys";
import { popupIsOpen } from "@armada/screens/src/list-keyboard";

import { useReviewStore, useReviews, type ReviewStore } from "./store";

const ZONES = [".pr-list", ".pr-reader", ".deck-controls"] as const;
const zoneOf = (target: Element): (typeof ZONES)[number] | undefined => ZONES.find((one) => target.closest(one) !== null);

/** What the arrows move between in the reader, in the order it is drawn. */
const STOPS = '.pr-explainer [role="tab"][tabindex="0"], .pr-desc > header button, .pr-asked__words, article[data-stop]';

const all = (selector: string, within: ParentNode = document) => [...within.querySelectorAll<HTMLElement>(selector)].filter((one) => !one.hasAttribute("disabled"));

/** Moves focus along `items` from the one holding it (or the one the key was pressed on), without wrapping. */
function step(items: HTMLElement[], from: Element, by: 1 | -1): void {
  const at = items.findIndex((one) => one === from || one.contains(from));
  items[at < 0 ? (by === 1 ? 0 : items.length - 1) : Math.min(items.length - 1, Math.max(0, at + by))]?.focus();
}

function focusList(): void {
  const rows = all(".pr-list .pr-row");
  (rows.find((one) => one.hasAttribute("data-selected")) ?? rows[0] ?? document.querySelector<HTMLElement>(".pr-list .pr-list__pick button") ?? document.querySelector<HTMLElement>(".pr-list"))?.focus();
}
function focusReader(): void {
  document.querySelector<HTMLElement>(".pr-reader")?.focus();
}
function focusBar(): void {
  (document.querySelector<HTMLElement>('.pr-explainer [role="tab"][tabindex="0"]') ?? document.querySelector<HTMLElement>(".pr-reader"))?.focus();
}
function focusTools(): void {
  const tiles = all(".deck-controls .pr-tool");
  (tiles.find((one) => one.getAttribute("aria-pressed") === "true") ?? tiles[0])?.focus();
}
const FOCUS = { ".pr-list": focusList, ".pr-reader": focusReader, ".deck-controls": focusTools } as const;

/**
 * The keys of the review surface, mounted with its foot tools, which are on screen wherever the review is. Takes a press in the
 * capture phase as the deck does, so a walk's own keys do not hear it first.
 */
export function ReviewKeys() {
  const store = useReviewStore()!;
  const reviews = useReviews();
  useEffect(() => {
    const listen = (event: KeyboardEvent) => press(event, store);
    window.addEventListener("keydown", listen, true);
    return () => window.removeEventListener("keydown", listen, true);
  }, [store]);
  // Natural Search takes focus in its box when it is put up.
  useEffect(() => {
    if (reviews.searching) window.requestAnimationFrame(() => document.querySelector<HTMLElement>('.pr-find textarea')?.focus());
  }, [reviews.searching]);
  return null;
}

function press(event: KeyboardEvent, store: ReviewStore): void {
  // A dialog's keys are its own while it is up.
  if (document.querySelector('[role="dialog"], [role="alertdialog"]') !== null) return;
  // So is an open menu's or list's: Esc closes that first, wherever focus is.
  if (event.key === "Escape" && popupIsOpen()) return;
  if (event.metaKey || event.ctrlKey || event.altKey) return;
  if (!(event.target instanceof Element)) return;
  const target = event.target;
  const snap = store.get();
  const here = zoneOf(target);
  const pr = snap.prs.find((one) => one.key === snap.open);

  // The first Esc leaves a box, as on the deck. The next one backs out a level.
  if (holdsText(target)) {
    if (isPressed("close", event) && here !== undefined) {
      event.preventDefault();
      (target as HTMLElement).blur();
      (here === ".pr-list" ? focusList : FOCUS[here])();
    }
    return;
  }
  if (target.closest('[role="menu"]') !== null) return;

  const take = () => {
    event.preventDefault();
    event.stopPropagation();
  };

  // Esc backs out a level: the box, a tool's results, a part, and stops at the list. **It never closes the open pull
  // request**: only opening another one changes what the reader holds.
  if (isPressed("close", event)) {
    if (here === undefined && document.querySelector(".pr-surface") === null) return;
    if (snap.searching) {
      take();
      store.setSearching(false);
      return focusList();
    }
    // Esc leaves the quick actions bar for the reader, and only then backs out further.
    if (target.closest(".pr-explainer") !== null && target.closest('[role="tab"], .pr-asked__words') !== null) {
      take();
      return document.querySelector<HTMLElement>(".pr-reader")?.focus();
    }
    if (pr !== undefined && (snap.work[pr.key]?.view ?? "explanation") !== "explanation" && here !== ".pr-list") {
      take();
      return store.setView(pr.key, "explanation");
    }
    if (here === ".pr-reader" || here === ".deck-controls") {
      take();
      return focusList();
    }
    return;
  }

  if (event.key === "F6") {
    take();
    const at = here === undefined ? -1 : ZONES.indexOf(here);
    const to = ZONES[(at + (event.shiftKey ? -1 : 1) + ZONES.length) % ZONES.length]!;
    return FOCUS[to]();
  }

  // Which part: the same letters the deck uses for the same places.
  const part = pressedSlot("review_panel", event);
  if (part === 0) return take(), focusList();
  if (part === 1) return take(), focusReader();
  if (part === 2) return take(), focusTools();
  if (isPressed("review_lists", event)) {
    take();
    return document.querySelector<HTMLElement>(".pr-list__pick button")?.click();
  }
  if (isPressed("review_search", event)) {
    take();
    return store.setSearching(true);
  }

  if (isPressed("review_where", event)) {
    take();
    return store.showPlaces();
  }

  // The quick actions bar takes one key, and the description its own, for the pull request that is open.
  if (pr !== undefined) {
    if (isPressed("review_quick", event)) {
      take();
      return focusBar();
    }
    if (isPressed("review_ask", event)) {
      take();
      return document.querySelector<HTMLElement>('.pr-ask-bar [role="textbox"]')?.focus();
    }
    if (isPressed("review_description", event)) {
      take();
      return document.querySelector<HTMLElement>(".pr-desc > header button")?.click();
    }
  }

  const slot = pressedSlot("move_focus", event);
  const vertical = slot !== -1 || event.key === "ArrowDown" || event.key === "ArrowUp";
  const down = slot === 0 || slot === 2 || event.key === "ArrowDown";
  const sideways = event.key === "ArrowLeft" || event.key === "ArrowRight";
  const by = down ? 1 : -1;

  if (here === ".pr-list") {
    // The dropdown's own button keeps its keys.
    if (target.closest(".pr-list__pick") !== null) return;
    if (vertical) {
      take();
      return step(all(".pr-list .pr-row"), target, by);
    }
    // Enter and `o` open the row under focus, as on the deck's list.
    if ((event.key === "Enter" || isPressed("open", event)) && target.closest(".pr-row") !== null) {
      take();
      return target.closest<HTMLElement>(".pr-row")!.click();
    }
    return;
  }

  if (here === ".pr-reader") {
    // **The bar**: its actions, then the questions asked, are one run for j and k and the arrows. ↵ or `o` chooses, and Esc leaves it.
    const inBar = target.closest(".pr-explainer") !== null && target.closest('[role="tab"], .pr-asked__words') !== null;
    if (inBar) {
      const run = all('.pr-explainer [role="tab"], .pr-explainer .pr-asked__words');
      const at = run.findIndex((one) => one === target.closest('[role="tab"], .pr-asked__words'));
      const tabs = all('.pr-explainer [role="tab"]');
      // Left and right stay along the actions (on a question they move between its own buttons, below). Past the last of the run, down goes on to the description.
      if (vertical || (sideways && at < tabs.length)) {
        take();
        const next = vertical ? at + by : Math.min(tabs.length - 1, Math.max(0, at + (event.key === "ArrowRight" ? 1 : -1)));
        if (next >= run.length) return document.querySelector<HTMLElement>(".pr-desc > header button")?.focus();
        return run[Math.max(0, next)]?.focus();
      }
      if (event.key === "Enter" || event.key === " " || isPressed("open", event)) {
        take();
        return target.closest<HTMLElement>('[role="tab"], .pr-asked__words')!.click();
      }
    }
    // A question's row: its words, Run again, Remove.
    const row = target.closest(".pr-asked__row");
    if (row !== null) {
      if (sideways) {
        take();
        return step(all("button", row), target, event.key === "ArrowRight" ? 1 : -1);
      }
      if (event.key === "Delete" || event.key === "Backspace") {
        take();
        return row.querySelector<HTMLElement>('button[aria-label="Remove this question"]')?.click();
      }
    }
    if (vertical) {
      take();
      // A question's own buttons stand for its row.
      const from = target.closest(".pr-asked__row")?.querySelector<HTMLElement>(".pr-asked__words") ?? target;
      return step(all(STOPS), from, by);
    }
    if (event.key === "Enter" || event.key === " ") {
      const control = target.closest<HTMLElement>("button, [role='tab']");
      // A change opens its hunk, and a control is pressed (a key a walk sends does not press it on its own).
      if (control === null && target.matches("article[data-stop]")) {
        take();
        return target.querySelector<HTMLElement>(".pr-change__toggle, [data-run]")?.click();
      }
      if (control !== null && !target.closest(".pr-find")) {
        take();
        return control.click();
      }
    }
    return;
  }

  if (here === ".deck-controls") {
    if (vertical || sideways) {
      take();
      const tiles = all(".deck-controls .pr-tool");
      return step(tiles, target, sideways ? (event.key === "ArrowRight" ? 1 : -1) : by);
    }
    if (event.key === "Enter" && target.closest("button") !== null) {
      take();
      return target.closest<HTMLElement>("button")!.click();
    }
  }
}
