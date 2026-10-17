// The repository field at the head of the Cockpit's left column, as a combobox: the WAI-ARIA pattern with a listbox
// popup. Pressing or focusing the field makes it a text box in place, the current pick shown as its placeholder; the list
// opens under it and narrows as he types. Arrows move through it, Enter picks, Esc puts the previous value back and
// closes. All repositories leads, a rule under it (no count beside it); the repositories follow; Add a repository is a footer action, outside the
// filter and reachable by arrow like any row.

import { useId, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { createPortal } from "react-dom";
import { Check, ChevronDown, FolderGit2, Layers, Plus } from "lucide-react";
import type { RepositorySummary } from "@armada/protocol";
import { repositoryLabel } from "@armada/shell";

const ALL_ROW = "all";
const ADD_ROW = "add";

type Row = { readonly id: string; readonly root: string | null; readonly label: string };

export function RepositoryCombobox({
  repositories,
  picked,
  onPick,
  onAdd,
}: {
  repositories: readonly RepositorySummary[];
  /** The root picked, or null for All. */
  picked: RepositorySummary | null;
  onPick: (root: string | null) => void;
  onAdd?: (() => void) | undefined;
}) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const [box, setBox] = useState<{ left: number; top: number; width: number }>({ left: 0, top: 0, width: 0 });
  const listId = useId();
  const current = picked === null ? "All repositories" : repositoryLabel(picked, repositories);
  const needle = query.trim().toLowerCase();

  const rows = useMemo<Row[]>(() => {
    const all: Row[] = [{ id: ALL_ROW, root: null, label: "All repositories" }];
    const repos: Row[] = repositories.map((one) => ({ id: one.root, root: one.root, label: repositoryLabel(one, repositories) }));
    return [...all, ...repos].filter((row) => needle === "" || row.label.toLowerCase().includes(needle));
  }, [repositories, needle]);
  // The arrow order: the filtered rows, then the footer action, which the filter never removes.
  const order = onAdd === undefined ? rows.map((row) => row.id) : [...rows.map((row) => row.id), ADD_ROW];
  const activeId = open && order.length > 0 ? order[Math.min(active, order.length - 1)] : undefined;

  const show = () => {
    const rect = input.current?.getBoundingClientRect();
    if (rect !== undefined) setBox({ left: rect.left, top: rect.bottom + 4, width: rect.width });
    setActive(0);
    setOpen(true);
  };
  const close = () => {
    setOpen(false);
    setQuery("");
  };
  const choose = (id: string) => {
    close();
    if (id === ADD_ROW) return onAdd?.();
    const row = rows.find((one) => one.id === id);
    if (row !== undefined) onPick(row.root);
  };
  const keys = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      if (!open) return show();
      const step = event.key === "ArrowDown" ? 1 : -1;
      setActive((at) => (Math.min(at, order.length - 1) + step + order.length) % order.length);
    } else if (event.key === "Enter") {
      if (!open) return;
      event.preventDefault();
      if (activeId !== undefined) choose(activeId);
    } else if (event.key === "Escape") {
      if (!open) return;
      event.preventDefault();
      event.stopPropagation();
      close();
    } else if (event.key === "Tab") {
      close();
    }
  };

  const optionId = (id: string) => `${listId}-${id === ALL_ROW || id === ADD_ROW ? id : `r${repositories.findIndex((one) => one.root === id)}`}`;
  const option = (row: Row) => (
    <li
      key={row.id}
      id={optionId(row.id)}
      role="option"
      aria-selected={(picked === null ? row.root === null : row.root === picked.root) || undefined}
      data-active={activeId === row.id || undefined}
      data-scope={row.root === null || undefined}
      className="deck-combo__option"
      onMouseDown={(event) => event.preventDefault()}
      onMouseMove={() => setActive(order.indexOf(row.id))}
      onClick={() => choose(row.id)}
    >
      {row.root === null ? <Layers size={16} aria-hidden="true" /> : <FolderGit2 size={16} aria-hidden="true" />}
      <span className="deck-combo__label">{row.label}</span>
      {(picked === null ? row.root === null : row.root === picked.root) ? <Check className="deck-combo__check" size={16} aria-hidden="true" /> : null}
    </li>
  );
  const [first, ...rest] = rows;

  return (
    <div className="deck-combo" data-open={open || undefined}>
      <input
        ref={input}
        type="text"
        role="combobox"
        className="deck-combo__input"
        aria-label="Repository"
        aria-expanded={open}
        aria-controls={listId}
        aria-autocomplete="list"
        aria-activedescendant={activeId === undefined ? undefined : optionId(activeId)}
        autoComplete="off"
        spellCheck={false}
        placeholder={current}
        value={open ? query : current}
        onFocus={show}
        onClick={() => (open ? undefined : show())}
        onBlur={close}
        onChange={(event) => {
          setQuery(event.target.value);
          setActive(0);
          if (!open) show();
        }}
        onKeyDown={keys}
      />
      <ChevronDown className="deck-combo__chevron" size={16} aria-hidden="true" />
      {open
        ? createPortal(
            <ul id={listId} role="listbox" aria-label="Repositories" className="deck-combo__list" style={{ left: box.left, top: box.top, width: box.width }}>
              {first === undefined ? <li role="presentation" className="deck-combo__empty">No repository matches</li> : first.root === null ? option(first) : null}
              {first !== undefined && first.root === null && rest.length > 0 ? <li role="presentation" className="deck-combo__rule" /> : null}
              {(first !== undefined && first.root === null ? rest : rows).map(option)}
              {onAdd === undefined ? null : (
                <>
                  <li role="presentation" className="deck-combo__rule" />
                  <li
                    id={optionId(ADD_ROW)}
                    role="option"
                    aria-selected={undefined}
                    data-active={activeId === ADD_ROW || undefined}
                    className="deck-combo__option deck-combo__add"
                    onMouseDown={(event) => event.preventDefault()}
                    onMouseMove={() => setActive(order.indexOf(ADD_ROW))}
                    onClick={() => choose(ADD_ROW)}
                  >
                    <Plus size={16} aria-hidden="true" />
                    <span className="deck-combo__label">Add a repository</span>
                  </li>
                </>
              )}
            </ul>,
            document.body,
          )
        : null}
    </div>
  );
}
