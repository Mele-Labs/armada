// The Projects tab's keys, so every act on it is reachable without the mouse. **They are acts of `actions.toml`**
// (`projects_*`, scope `projects`), so Settings → Keyboard lists and rebinds them. The list motions (`move_focus`, `open`,
// `close`) are the registry's `list` rows, as on Issues. Two parts, the left column and the cards of what is open, a key to
// each; Esc backs out a level, the search's words first. `/` goes to the search field. A key stands down in a text field and
// while a dialog is up. The column's stops are the three views, the projects and the repositories, each of which opens its cards.

import { useEffect, useRef } from "react";
import { forgetKept, isPressed, pressedSlot, readKept } from "@armada/components";
import { holdsText } from "@armada/screens/src/keys";
import { popupIsOpen } from "@armada/screens/src/list-keyboard";

import { allRows, useProjectsStore, type ProjectsStore } from "./store";
import { SEARCH_KEEP, VIEW_IDS, type View } from "./views";

const ZONES = [".projects-list", ".projects-pane"] as const;
const zoneOf = (target: Element): (typeof ZONES)[number] | undefined => ZONES.find((one) => target.closest(one) !== null);

/** The column's stops, in the order drawn: the views, the projects, the repositories. */
const STOPS = ".projects-list .projects-item";
/** What j and k pass along the pane: its cards. */
const PANE_STOPS = ".projects-pane .projects-row";

const all = (selector: string) => [...document.querySelectorAll<HTMLElement>(selector)].filter((one) => !one.hasAttribute("disabled"));

/** Moves focus along `items` from the one holding it, without wrapping. */
function step(items: HTMLElement[], from: Element, by: 1 | -1): void {
  const at = items.findIndex((one) => one === from || one.contains(from));
  items[at < 0 ? (by === 1 ? 0 : items.length - 1) : Math.min(items.length - 1, Math.max(0, at + by))]?.focus();
}

function focusList(): void {
  const items = all(STOPS);
  (items.find((one) => one.hasAttribute("data-selected")) ?? items[0] ?? document.querySelector<HTMLElement>(".projects-list"))?.focus();
}
/** The column's stop once the search is gone and the view that was open is marked again. */
const clearedList = (): void => void window.requestAnimationFrame(focusList);
function focusRows(): void {
  (document.querySelector<HTMLElement>(".projects-pane .projects-row") ?? document.querySelector<HTMLElement>(".projects-pane"))?.focus();
}
/** The row a moved row is drawn at once the list has drawn again. */
const refocus = (rowId: string): void => void window.requestAnimationFrame(() => document.querySelector<HTMLElement>(`.projects-row[data-row="${CSS.escape(rowId)}"]`)?.focus());

export function ProjectsKeys({ onDispatchRow, onView }: { onDispatchRow: (rowId: string) => void; onView: (view: View) => void }) {
  const store = useProjectsStore()!;
  const latest = useRef({ onDispatchRow, onView });
  latest.current = { onDispatchRow, onView };
  useEffect(() => {
    const listen = (event: KeyboardEvent) => press(event, store, (id) => latest.current.onDispatchRow(id), (view) => latest.current.onView(view));
    window.addEventListener("keydown", listen, true);
    return () => window.removeEventListener("keydown", listen, true);
  }, [store]);
  return null;
}

function press(event: KeyboardEvent, store: ProjectsStore, dispatchRow: (rowId: string) => void, goToView: (view: View) => void): void {
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

  // The search field: Esc clears the words and goes back to the view that was open, ↓ goes to the first result.
  if (holdsText(target) && target.closest(".projects-list [data-search-field]") !== null) {
    if (isPressed("close", event)) {
      take();
      (target as HTMLElement).blur();
      forgetKept(SEARCH_KEEP);
      return clearedList();
    }
    if (event.key === "ArrowDown") {
      take();
      focusRows();
    }
    return;
  }

  // The first Esc leaves a field, and the field closes with it: a subtask field, a title or a name being
  // edited goes back as it was; the task field folds if it holds no words. Focus lands where the field was opened from. The next Esc backs out a level.
  if (holdsText(target)) {
    if (isPressed("close", event) && here !== undefined) {
      take();
      (target as HTMLElement).blur();
      const card = target.closest<HTMLElement>(".projects-row")?.dataset.row;
      if (target.closest(".projects-add[data-sub]") !== null) store.subtasking(undefined);
      if (target.closest(".projects-row__title") !== null) store.editing(undefined);
      if (target.closest(".projects-name") !== null) {
        store.name(false);
        return focusList();
      }
      if (target.closest(".projects-rename") !== null) {
        const renamed = snap.renaming;
        store.renaming(undefined);
        return void window.requestAnimationFrame(() => (renamed === undefined ? focusList() : document.querySelector<HTMLElement>(`.projects-item[data-project="${CSS.escape(renamed)}"]`)?.focus()));
      }
      if (card !== undefined) return refocus(card);
      focusRows();
    }
    return;
  }
  if (target.closest('[role="menu"]') !== null) return;

  const row = target.closest<HTMLElement>(".projects-pane .projects-row");
  const rowId = row?.dataset.row;
  const paned = document.querySelector(".projects-pane") !== null;
  const rowEntry = rowId === undefined ? undefined : allRows({ rows: [...snap.loose, ...snap.projects.flatMap((one) => one.rows)] }).find((one) => one.id === rowId);
  /** Focus lands on the card after this one, else the one before, once it has left the list. */
  const neighbour = (card: HTMLElement) => {
    const cards = all(PANE_STOPS);
    const at = cards.indexOf(card);
    return cards.slice(at + 1).find((one) => !card.parentElement?.contains(one)) ?? cards[at - 1];
  };

  // ⌥↑ and ⌥↓ move the row under focus, as the deck moves its own.
  const shove = event.altKey && !event.metaKey && !event.ctrlKey ? pressedSlot("projects_reorder", event) : -1;
  if (shove !== -1) {
    // Only a project's own cards are in his order; a view's or a repository's are not moved.
    if (!paned || rowId === undefined || row?.hasAttribute("data-fixed") === true) return;
    take();
    store.move(rowId, shove === 0 ? -1 : 1);
    return refocus(rowId);
  }
  if (event.metaKey || event.ctrlKey || event.altKey) return;

  if (isPressed("close", event)) {
    // Words in the search are the first thing Esc puts away.
    if (here !== undefined && (readKept<string>(SEARCH_KEEP) ?? "").trim() !== "") {
      take();
      forgetKept(SEARCH_KEEP);
      return clearedList();
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
  const view = pressedSlot("projects_view", event);
  if (view !== -1 && VIEW_IDS[view] !== undefined) return take(), goToView(VIEW_IDS[view]!);
  if (isPressed("projects_search", event)) {
    const field = document.querySelector<HTMLElement>(".projects-list [data-search-field]");
    if (field === null) return;
    take();
    return field.focus();
  }
  if (isPressed("projects_new", event)) return take(), store.name(true);
  if (paned && isPressed("projects_done", event)) return take(), store.showDone(!snap.showDone);
  if (paned && isPressed("projects_add_task", event)) {
    // The pane's field, where it has one.
    const field = document.querySelector<HTMLElement>(".projects-top .projects-add textarea");
    if (field === null) return;
    take();
    return field.focus();
  }
  if (paned && isPressed("projects_dispatch", event)) {
    // Cards picked: the head's Dispatch, which is drawn only then. Otherwise the focused card.
    const button = document.querySelector<HTMLElement>(".projects-pane [data-dispatch]");
    if (button !== null) return take(), button.click();
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
    if (isPressed("projects_delete", event)) {
      take();
      // Focus goes to the card that takes its place, or the one above where it was the last.
      const next = neighbour(row);
      store.remove(rowId);
      return next?.dataset.row === undefined ? undefined : refocus(next.dataset.row);
    }
    if (isPressed("projects_tick", event)) {
      take();
      // A task ticked done leaves the list while done tasks are hidden, so focus goes to its neighbour.
      const leaves = rowEntry?.kind === "task" && !rowEntry.done && !snap.showDone;
      const next = leaves ? neighbour(row) : undefined;
      store.tick(rowId);
      return next?.dataset.row === undefined ? (leaves ? focusRows() : undefined) : refocus(next.dataset.row);
    }
    // Enter edits a task's title in place; an issue's is its tracker's.
    if (isPressed("projects_edit", event) && target === row) {
      if (rowEntry?.kind !== "task") return;
      take();
      return store.editing(rowId);
    }
    if (isPressed("projects_open_job", event)) {
      const job = row.querySelector<HTMLElement>("[data-job]");
      if (job === null) return;
      take();
      return job.click();
    }
    if (isPressed("projects_unlink_job", event)) {
      if (row.querySelector("[data-job]") === null) return;
      take();
      return store.unlinkJob(rowId);
    }
    if (isPressed("projects_unlink_pull", event)) {
      const pulls = row.querySelectorAll<HTMLElement>("[data-unlink-pull]");
      const last = pulls[pulls.length - 1];
      if (last === undefined) return;
      take();
      return last.click();
    }
    if (isPressed("projects_mark_done", event)) {
      const done = row.querySelector<HTMLElement>("[data-mark-done]");
      if (done === null) return;
      take();
      const next = snap.showDone ? undefined : neighbour(row);
      store.settle(rowId, true);
      return next?.dataset.row === undefined ? (snap.showDone ? undefined : focusRows()) : refocus(next.dataset.row);
    }
    if (isPressed("projects_keep_open", event)) {
      if (row.querySelector("[data-keep-open]") === null) return;
      take();
      return store.settle(rowId, false);
    }
  }

  const slot = pressedSlot("move_focus", event);
  const vertical = slot !== -1 || event.key === "ArrowDown" || event.key === "ArrowUp";
  const by = slot === 0 || slot === 2 || event.key === "ArrowDown" ? 1 : -1;

  if (here === ".projects-list") {
    const item = target.closest<HTMLElement>(".projects-item");
    const project = item?.dataset.project;
    if (project !== undefined && isPressed("projects_rename", event)) return take(), store.renaming(project);
    if (project !== undefined && isPressed("projects_delete", event)) {
      take();
      const items = all(STOPS);
      const next = items[items.indexOf(item!) + 1] ?? items[items.indexOf(item!) - 1];
      store.removeProject(project);
      return next?.focus();
    }
    if (vertical) {
      take();
      return step(all(STOPS), target, by);
    }
    const stop = target.closest<HTMLElement>(".projects-item");
    if ((event.key === "Enter" || isPressed("open", event)) && stop !== null) {
      take();
      return stop.click();
    }
    return;
  }

  if (here === ".projects-pane" && vertical) {
    take();
    return step(all(PANE_STOPS), target, by);
  }
}
