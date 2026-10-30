// The Issues tab's keys, so every act on it is reachable without the mouse. **They are acts of `actions.toml`** (`issues_*`,
// scope `issues`), so Settings → Keyboard lists and rebinds them. The list motions (`move_focus`, `open`, `close`) are the
// registry's `list` rows, as on Reviews. Two parts, the list and the reader, a key to each; Esc backs out a level. A key stands
// down in a text field and while a dialog is up.

import { useEffect, useRef } from "react";
import { isPressed, pressedSlot } from "@armada/components";
import { holdsText } from "@armada/screens/src/keys";
import { popupIsOpen } from "@armada/screens/src/list-keyboard";

import { useIssues, useIssuesStore, type IssuesStore } from "./store";

const ZONES = [".issues-list", ".issues-reader"] as const;
const zoneOf = (target: Element): (typeof ZONES)[number] | undefined => ZONES.find((one) => target.closest(one) !== null);

const all = (selector: string) => [...document.querySelectorAll<HTMLElement>(selector)].filter((one) => !one.hasAttribute("disabled"));

/** Moves focus along `items` from the one holding it, without wrapping. */
function step(items: HTMLElement[], from: Element, by: 1 | -1): void {
  const at = items.findIndex((one) => one === from || one.contains(from));
  items[at < 0 ? (by === 1 ? 0 : items.length - 1) : Math.min(items.length - 1, Math.max(0, at + by))]?.focus();
}

function focusList(): void {
  const rows = all(".issues-list .issues-row");
  (rows.find((one) => one.hasAttribute("data-selected")) ?? rows[0] ?? document.querySelector<HTMLElement>(".issues-list .issues-refused__teach") ?? document.querySelector<HTMLElement>(".issues-list__pick button") ?? document.querySelector<HTMLElement>(".issues-list"))?.focus();
}
function focusReader(): void {
  document.querySelector<HTMLElement>(".issues-reader")?.focus();
}

export function IssuesKeys() {
  const store = useIssuesStore()!;
  const { describing } = useIssues();
  useEffect(() => {
    const listen = (event: KeyboardEvent) => press(event, store);
    window.addEventListener("keydown", listen, true);
    return () => window.removeEventListener("keydown", listen, true);
  }, [store]);
  // The box takes focus when it is put up, and the list when a query has been written for him.
  const was = useRef(describing?.phase);
  useEffect(() => {
    if (describing?.phase === "box") window.requestAnimationFrame(() => document.querySelector<HTMLElement>(".issues-describe textarea")?.focus());
    else if (was.current === "box" && describing?.phase === "written") window.requestAnimationFrame(focusList);
    was.current = describing?.phase;
  }, [describing?.phase]);
  return null;
}

function press(event: KeyboardEvent, store: IssuesStore): void {
  if (document.querySelector(".issues-surface") === null) return;
  if (document.querySelector('[role="dialog"], [role="alertdialog"]') !== null) return;
  if (event.key === "Escape" && popupIsOpen()) return;
  if (event.metaKey || event.ctrlKey || event.altKey) return;
  if (!(event.target instanceof Element)) return;
  const target = event.target;
  const snap = store.get();
  const here = zoneOf(target);
  const take = () => {
    event.preventDefault();
    event.stopPropagation();
  };

  // The first Esc leaves a box, as on Reviews; the describe box closes with it. The next one backs out a level.
  if (holdsText(target)) {
    // The search bar clears its words on the first Esc, in its own handler, and is left on the next.
    if (target.matches("[data-search-field]") && (target as HTMLInputElement).value !== "") return;
    if (isPressed("close", event) && here !== undefined) {
      take();
      (target as HTMLElement).blur();
      if (target.closest(".issues-describe") !== null) store.stopDescribing();
      if (target.closest(".issues-query--editing") !== null) store.edit(undefined);
      focusList();
    }
    return;
  }
  if (target.closest('[role="menu"]') !== null) return;

  if (isPressed("close", event)) {
    if (snap.describing !== undefined) {
      take();
      store.stopDescribing();
      return focusList();
    }
    if (snap.editing !== undefined) {
      take();
      store.edit(undefined);
      return focusList();
    }
    if (here === ".issues-reader") {
      take();
      return focusList();
    }
    if (snap.open !== undefined || snap.openReport !== undefined) {
      take();
      return store.open(undefined);
    }
    return;
  }

  const part = pressedSlot("issues_panel", event);
  if (part === 0) return take(), focusList();
  if (part === 1) return take(), focusReader();
  if (isPressed("issues_queries", event)) {
    take();
    return document.querySelector<HTMLElement>(".issues-list__pick button")?.click();
  }
  if (isPressed("issues_search", event)) {
    const field = document.querySelector<HTMLElement>(".issues-list [data-search-field]");
    if (field === null) return;
    take();
    return field.focus();
  }
  // A reported bug: the one in the reader, or the row under focus in the list.
  const reported = here === ".issues-reader" ? snap.openReport : here === ".issues-list" ? target.closest<HTMLElement>("[data-report]")?.dataset.report : undefined;
  const report = snap.reports.find((one) => one.id === reported);
  if (report !== undefined && isPressed("issues_drop", event)) {
    take();
    void store.dropReport(report.id).then(() => window.requestAnimationFrame(focusList));
    return;
  }
  if (report !== undefined && !report.kept && isPressed("issues_keep", event)) {
    take();
    void store.keepReport(report.id).then(() => window.requestAnimationFrame(here === ".issues-reader" ? focusReader : focusList));
    return;
  }
  if (isPressed("issues_describe", event)) {
    take();
    return store.describe();
  }
  if (isPressed("issues_start", event)) {
    const start = document.querySelector<HTMLElement>(".issues-reader [data-start]");
    if (start === null) return;
    take();
    return start.click();
  }

  const slot = pressedSlot("move_focus", event);
  const vertical = slot !== -1 || event.key === "ArrowDown" || event.key === "ArrowUp";
  const by = slot === 0 || slot === 2 || event.key === "ArrowDown" ? 1 : -1;

  if (here === ".issues-list") {
    if (target.closest(".issues-list__pick") !== null) return;
    if (vertical) {
      take();
      return step(all(".issues-list .issues-row, .issues-list .issues-refused__teach"), target, by);
    }
    // Enter and `o` open the row under focus. Focus stays on the list, so j and k go on through it.
    if ((event.key === "Enter" || isPressed("open", event)) && target.closest(".issues-row") !== null) {
      take();
      return target.closest<HTMLElement>(".issues-row")!.click();
    }
    return;
  }

  if (here === ".issues-reader" && vertical) {
    take();
    return step(all(".issues-reader button"), target, by);
  }
}
