// The repository field at the head of the Cockpit's left column, as a combobox: the WAI-ARIA pattern with a listbox
// popup. Pressing or focusing the field makes it a text box in place, the current pick shown as its placeholder; the list
// opens under it and narrows as he types. Arrows move through it, Enter picks, Esc puts the previous value back and
// closes. All repositories leads, a rule under it (no count beside it); the repositories follow; Add a repository is a footer action, outside the
// filter and reachable by arrow like any row. A repository whose runs Fleet keeps wears the glyph of the place they are kept in: → on its row, or a press
// on the glyph, opens all of how it runs as a layer. Mock only.

import { useId, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { createPortal } from "react-dom";
import { Check, ChevronDown, FolderGit2, Layers, Plus } from "lucide-react";
import { PLACE_GLYPH, RunsAll, RunsPlaces, Tooltip, placePhrase } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import { repositoryLabel } from "@armada/shell";

import { useSessions, useSessionsDraft } from "../../sessions-draft";

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
  // The repository whose runs are open as a layer, and whether its place is being changed.
  const [layer, setLayer] = useState<string | null>(null);
  const [changing, setChanging] = useState(false);
  const draft = useSessionsDraft();
  useSessions();
  const runsOf = (root: string | null) => (root === null ? undefined : draft?.runs?.of(root));
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
  const openRuns = (root: string) => {
    close();
    setChanging(false);
    setLayer(root);
  };
  const closeRuns = () => {
    setLayer(null);
    setChanging(false);
  };
  const keys = (event: KeyboardEvent<HTMLInputElement>) => {
    const row = open && activeId !== undefined && event.key === "ArrowRight" ? rows.find((one) => one.id === activeId) : undefined;
    if (row?.root != null && runsOf(row.root) !== undefined && event.currentTarget.selectionStart === query.length) {
      event.preventDefault();
      return openRuns(row.root);
    }
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
  /** The glyph of the place a repository's runs are kept in, which opens all of how it runs. */
  const mark = (row: Row) => {
    const kept = runsOf(row.root);
    if (kept === undefined || row.root === null) return null;
    const Glyph = PLACE_GLYPH[kept.place.id];
    const root = row.root;
    return (
      <Tooltip label={`Kept ${placePhrase(kept.place)}`} shortcut="→">
        <button
          type="button"
          tabIndex={-1}
          className="deck-combo__mark"
          aria-label={`How ${kept.repository} runs`}
          onMouseDown={(event) => event.preventDefault()}
          onClick={(event) => {
            event.stopPropagation();
            openRuns(root);
          }}
        >
          <Glyph size={12} aria-hidden="true" />
        </button>
      </Tooltip>
    );
  };
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
      {mark(row)}
      {(picked === null ? row.root === null : row.root === picked.root) ? <Check className="deck-combo__check" size={16} aria-hidden="true" /> : null}
    </li>
  );
  const [first, ...rest] = rows;

  const shown = layer === null ? undefined : runsOf(layer);
  const places = draft?.runs;

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
      {layer === null || shown === undefined || places === undefined
        ? null
        : createPortal(
            <div className="deck-runs-layer" onPointerDown={(event) => event.target === event.currentTarget && closeRuns()}>
              <RunsAll
                name={shown.repository}
                source={shown.source}
                rows={shown.rows}
                place={shown.place}
                onDrop={(id) => places.drop(layer, id)}
                changing={changing}
                onChange={() => setChanging(!changing)}
                chooser={
                  !changing ? null : (
                    <RunsPlaces
                      synced={places.synced}
                      profiles={places.profiles}
                      current={shown.place}
                      onPick={(place) => {
                        places.place(layer, place);
                        setChanging(false);
                      }}
                      onClose={() => setChanging(false)}
                    />
                  )
                }
                onClose={closeRuns}
              />
            </div>,
            document.body,
          )}
    </div>
  );
}
