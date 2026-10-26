// The Projects tab's keys, so every act on it is reachable without the mouse. **They are acts of `actions.toml`**
// (`projects_*`, scope `projects`), so Settings → Keyboard lists and rebinds them. The list motions (`move_focus`, `open`,
// `close`) are the registry's `list` rows, as on Issues. Two parts, the list of projects and the open project's rows, a key to
// each; Esc backs out a level. A key stands down in a text field and while a dialog is up.

import { useEffect, useRef } from "react";
import { isPressed, pressedSlot } from "@armada/components";
import { holdsText } from "@armada/screens/src/keys";
import { popupIsOpen } from "@armada/screens/src/list-keyboard";

import { useProjectsStore, type ProjectsStore } from "./store";

const ZONES = [".projects-list", ".projects-pane"] as const;
const zoneOf = (target: Element): (typeof ZONES)[number] | undefined => ZONES.find((one) => target.closest(one) !== null);

const all = (selector: string) => [...document.querySelectorAll<HTMLElement>(selector)].filter((one) => !one.hasAttribute("disabled"));

/** Moves focus along `items` from the one holding it, without wrapping. */
function step(items: HTMLElement[], from: Element, by: 1 | -1): void {
  const at = items.findIndex((one) => one === from || one.contains(from));
  items[at < 0 ? (by === 1 ? 0 : items.length - 1) : Math.min(items.length - 1, Math.max(0, at + by))]?.focus();
}

function focusList(): void {
  const items = all(".projects-list .projects-item");
  (items.find((one) => one.hasAttribute("data-selected")) ?? items[0] ?? document.querySelector<HTMLElement>(".projects-list"))?.focus();
}
function focusRows(): void {
  (document.querySelector<HTMLElement>(".projects-pane .projects-row") ?? document.querySelector<HTMLElement>(".projects-pane"))?.focus();
}
/** The row a moved row is drawn at once the list has drawn again. */
const refocus = (rowId: string): void => void window.requestAnimationFrame(() => document.querySelector<HTMLElement>(`.projects-row[data-row="${CSS.escape(rowId)}"]`)?.focus());

export function ProjectsKeys({ onDispatchRow }: { onDispatchRow: (rowId: string) => void }) {
  const store = useProjectsStore()!;
  const latest = useRef(onDispatchRow);
  latest.current = onDispatchRow;
  useEffect(() => {
    const listen = (event: KeyboardEvent) => press(event, store, (id) => latest.current(id));
    window.addEventListener("keydown", listen, true);
    return () => window.removeEventListener("keydown", listen, true);
  }, [store]);
  return null;
}

function press(event: KeyboardEvent, store: ProjectsStore, dispatchRow: (rowId: string) => void): void {
  if (document.querySelector(".projects-surface") === null) return;
  if (document.querySelector('[role="dialog"], [role="alertdialog"]') !== null) return;
  if (event.key === "Escape" && popupIsOpen()) return;
  if (!(event.target instanceof Element)) return;
  const target = event.target;
  const snap = store.get();
  const here = zoneOf(target);
  const take = () => {
    event.preventDefault();
    event.stopPropagation();
  };

  // The first Esc leaves a field, and the search and the name field close with it. The next one backs out a level.
  if (holdsText(target)) {
    if (isPressed("close", event) && here !== undefined) {
      take();
      (target as HTMLElement).blur();
      if (target.closest(".projects-link") !== null) store.linking(false);
      if (target.closest(".projects-add[data-sub]") !== null) store.subtasking(undefined);
      if (target.closest(".projects-name") !== null) {
        store.name(undefined);
        return focusList();
      }
      focusRows();
    }
    return;
  }
  if (target.closest('[role="menu"]') !== null) return;

  const row = target.closest<HTMLElement>(".projects-pane .projects-row");
  const rowId = row?.dataset.row;
  const open = snap.projects.find((one) => one.id === snap.open);

  // ⌥↑ and ⌥↓ move the row under focus, as the deck moves its own.
  const shove = event.altKey && !event.metaKey && !event.ctrlKey ? pressedSlot("projects_reorder", event) : -1;
  if (shove !== -1) {
    if (open === undefined || rowId === undefined) return;
    take();
    store.move(rowId, shove === 0 ? -1 : 1);
    return refocus(rowId);
  }
  if (event.metaKey || event.ctrlKey || event.altKey) return;

  if (isPressed("close", event)) {
    if (snap.linking) {
      take();
      store.linking(false);
      return focusRows();
    }
    if (here === ".projects-pane") {
      take();
      return focusList();
    }
    if (snap.open !== undefined) {
      take();
      return store.open(undefined);
    }
    return;
  }

  const part = pressedSlot("projects_panel", event);
  if (part === 0) return take(), focusList();
  if (part === 1) return take(), focusRows();
  if (isPressed("projects_new", event)) {
    take();
    return document.querySelector<HTMLElement>(".projects-list__new button")?.click();
  }
  if (open !== undefined && isPressed("projects_add_task", event)) {
    take();
    return document.querySelector<HTMLElement>(".projects-add textarea")?.focus();
  }
  if (open !== undefined && isPressed("projects_link", event)) {
    take();
    return store.linking(true);
  }
  if (open !== undefined && isPressed("projects_dispatch", event)) {
    if (snap.selected.length > 0) {
      take();
      return document.querySelector<HTMLElement>(".projects-pane [data-dispatch]")?.click();
    }
    if (rowId === undefined) return;
    take();
    return dispatchRow(rowId);
  }

  if (row !== null && rowId !== undefined) {
    if (isPressed("projects_select", event)) return take(), store.select(rowId);
    if (isPressed("projects_add_subtask", event)) {
      if (row.querySelector("[data-add-subtask]") === null) return;
      take();
      return store.subtasking(rowId);
    }
    if (isPressed("projects_link_task", event)) {
      if (row.querySelector("[data-link-task]") === null) return;
      take();
      return store.linking(true, rowId);
    }
    if (isPressed("projects_delete", event)) {
      take();
      // Focus goes to the card that takes its place, or the one above where it was the last.
      const rows = all(".projects-pane .projects-row");
      const at = rows.indexOf(row);
      const next = rows.slice(at + 1).find((one) => !row.parentElement?.contains(one)) ?? rows[at - 1];
      store.remove(rowId);
      return next?.dataset.row === undefined ? undefined : refocus(next.dataset.row);
    }
    if (isPressed("projects_tick", event)) return take(), store.tick(rowId);
    if (isPressed("projects_mark_done", event)) {
      const done = row.querySelector<HTMLElement>("[data-mark-done]");
      if (done === null) return;
      take();
      return store.settle(rowId, true);
    }
    if (isPressed("projects_keep_open", event)) {
      if (row.querySelector("[data-keep-open]") === null) return;
      take();
      return store.settle(rowId, false);
    }
    // Enter and `o` open the Job holding the row, where one does.
    if ((event.key === "Enter" || isPressed("open", event)) && target === row) {
      const job = row.querySelector<HTMLElement>("[data-job]");
      if (job === null) return;
      take();
      return job.click();
    }
  }

  const slot = pressedSlot("move_focus", event);
  const vertical = slot !== -1 || event.key === "ArrowDown" || event.key === "ArrowUp";
  const by = slot === 0 || slot === 2 || event.key === "ArrowDown" ? 1 : -1;

  if (here === ".projects-list") {
    if (vertical) {
      take();
      return step(all(".projects-list .projects-item"), target, by);
    }
    if ((event.key === "Enter" || isPressed("open", event)) && target.closest(".projects-item") !== null) {
      take();
      return target.closest<HTMLElement>(".projects-item")!.click();
    }
    return;
  }

  if (here === ".projects-pane" && vertical) {
    take();
    const rows = all(".projects-pane .projects-row");
    const hits = all(".projects-pane .projects-link__hit");
    return step([...rows, ...hits], target, by);
  }
}
