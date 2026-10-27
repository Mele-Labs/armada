import type { KeyboardEvent } from "react";

/**
 * The keys of a page drawn as columns, each marked `data-col` and holding rows marked `data-nav-item`.
 * Up and down move among a column's rows and pick the one they land on, right or Enter go into the
 * next column, left comes back, `/` reaches the search and Escape in it clears it.
 *
 * **A field keeps its own keys.** Arrows and Enter in a text box, a select or a slider do what
 * they do there, so a form in the last column is never walked out of by typing.
 */
const OWN_KEYS = "input, textarea, select, [contenteditable='true'], [role='combobox'], [role='slider'], [role='radio'], [role='switch']";
const REACH = "button:not([disabled]), input:not([disabled]), select:not([disabled]), a[href], [tabindex]:not([tabindex='-1'])";

const columnsOf = (root: HTMLElement): HTMLElement[] => [...root.querySelectorAll<HTMLElement>("[data-col]")];
const rowsOf = (column: HTMLElement): HTMLElement[] => [...column.querySelectorAll<HTMLElement>("[data-nav-item]")];

/** Where focus goes on entering a column: the row that is picked, else the first row, else the first control. */
function enter(column: HTMLElement): boolean {
  const rows = rowsOf(column);
  const picked = rows.find((one) => one.getAttribute("aria-current") !== null || one.getAttribute("aria-selected") === "true");
  const first = picked ?? rows[0] ?? column.querySelector<HTMLElement>(REACH);
  if (first === null || first === undefined) return false;
  first.focus();
  // Landing on a row picks it, so the column beside it follows. The last column's rows are acts, not picks.
  if (picked === undefined && rows[0] === first && column.dataset["col"] !== "detail") first.click();
  return true;
}

export function columnKeys(event: KeyboardEvent<HTMLElement>, root: HTMLElement | null): void {
  if (root === null || event.metaKey || event.ctrlKey || event.altKey) return;
  const target = event.target as HTMLElement;
  const typing = target.closest(OWN_KEYS) !== null;
  if (event.key === "/" && !typing) {
    const search = root.querySelector<HTMLInputElement>("[data-search-field]");
    if (search === null) return;
    event.preventDefault();
    search.focus();
    search.select();
    return;
  }
  if (typing) return;
  const columns = columnsOf(root);
  const column = target.closest<HTMLElement>("[data-col]");
  if (column === null) return;
  const row = target.closest<HTMLElement>("[data-nav-item]");
  const rows = rowsOf(column);
  const at = row === null ? -1 : rows.indexOf(row);
  const index = columns.indexOf(column);
  const into = (step: 1 | -1): boolean => {
    for (let i = index + step; i >= 0 && i < columns.length; i += step) if (enter(columns[i]!)) return true;
    return false;
  };

  if ((event.key === "ArrowDown" || event.key === "ArrowUp") && at >= 0) {
    const next = rows[at + (event.key === "ArrowDown" ? 1 : -1)];
    event.preventDefault();
    if (next === undefined) return;
    next.focus();
    if (column.dataset["col"] !== "detail") next.click();
  } else if (event.key === "ArrowRight" || (event.key === "Enter" && at >= 0)) {
    if (event.key === "ArrowRight" && column.dataset["col"] === "detail") return;
    const last = index === columns.length - 1;
    // Enter on a row in the last column is the row's own act.
    if (event.key === "Enter" && last) return;
    event.preventDefault();
    if (row !== null && event.key === "Enter") row.click();
    into(1);
  } else if (event.key === "ArrowLeft" && index > 0) {
    event.preventDefault();
    into(-1);
  }
}
