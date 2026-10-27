import { useEffect, useMemo, useRef, useState } from "react";
import type { LucideIcon } from "lucide-react";

import { isPressed } from "../../keymap";
import { KbdChord } from "../Kbd/Kbd";
import { ActionsPanel, ResultKeys, actionEntryId, narrowed, pressedChord, type PaletteAction } from "./actions-panel";
import { DISPATCH_ROW, DispatchKeys, DispatchRow, StepKeys, adds, nextKind, readsAsWork, type PaletteDispatch, type PaletteStep } from "./dispatch-row";
import { StateGlyph, Subtitle, type PaletteMeta } from "./row-meta";
import { usePlace } from "./place";

import { LINK_ROW, LinkRow, type PaletteLink } from "./link-row";
import { PATH_ROW, PathRow, readsAsPath, type PalettePath } from "./path-row";

export { readsAsPath, type PalettePath } from "./path-row";
export type { PaletteLink, PaletteLinkKind } from "./link-row";
export type { PaletteAction } from "./actions-panel";
export type { PaletteMeta } from "./row-meta";
export { adds, readsAsWork, type AddKind, type BarKind, type DispatchKind, type PaletteDispatch, type PaletteStep } from "./dispatch-row";

/**
 * The palette is a superset of the UI, never a substitute for it, and it is
 * the discovery surface: it is how a person learns forty shortcuts without a
 * cheat sheet, which is why every entry displays its binding and no action may
 * exist outside it.
 *
 * **The empty query is the cheat sheet, and that is the load-bearing state.**
 * Opening the palette with nothing typed shows every section head and every
 * binding, so reading three on the way past to do one thing is the whole
 * mechanism by which forty get learned without a help screen. Every other
 * state here is a narrowing of it.
 *
 * A floating layer — `--bg-overlay`, `--border-default`, `--radius-lg`, a
 * shadow, and no blur. **Top-anchored, never centered**: a centered dialog
 * rises as results fall away, so the row under the cursor changes while a
 * person is still typing.
 *
 * Three safety rules from `### Safety rules for single-key actions` are
 * structural here, not decoration.
 *
 * 1. Single-key shortcuts are suppressed whenever a text input holds focus.
 *    The palette is a text input, so every key except the navigation set and
 *    `Esc` goes to the query. Typing "axe" cannot approve, kill and open
 *    something.
 * 2. Every destructive action confirms, even from the keyboard. A destructive
 *    entry does not act and does not close the palette; it hands the entry to
 *    `onConfirm`, and the host opens the confirmation over it.
 * 3. Kill is `x`, never `k`. The map is `actions.toml`'s and this only draws
 *    what it is given.
 *
 * **The palette obeys the lexicon.** `aliases` are searched and never
 * rendered, so "terminate" finds Kill and the row still reads Kill — with no
 * marked span, because an alias has none to mark and faking one would render
 * the alias.
 */
export type PaletteEntry = {
  id: string;
  /** The id of the section it groups under. */
  section: string;
  /** The lexicon term. Always what renders. */
  label: string;
  /** Searched, never rendered. */
  aliases?: readonly string[];
  /**
   * The binding, where the entry has one.
   *
   * **Jobs and settings carry none, and that is not a gap.** A binding belongs
   * to an act; opening a specific job is a search result.
   */
  shortcut?: string;
  /**
   * The glyph, 16px, from `packages/icons/icons/` by way of `actions.ts`.
   *
   * **Absent is common and says nothing is wrong.** Thirteen acts carry no
   * registered glyph, so the slot holds its width and draws nothing rather
   * than the column going ragged or a silhouette being invented for it.
   */
  icon?: LucideIcon;
  /** A setting's current value, in mono, right of the label. */
  value?: string;
  /** Danger is the dropdown-menu treatment: red label and glyph, no fill. */
  destructive?: boolean;
  /**
   * Why this row cannot be chosen, in a few words, where it cannot be. The row
   * draws dimmed and says so beside its binding.
   *
   * **Two kinds of dormancy, one rendering.** A binding the registry carries
   * and nothing answers — `not built · #250`, from `unbuilt` — and an act this
   * app cannot reach right now, like copying debug info with no failure on
   * screen. Both are facts the caller holds; neither is a list of exceptions
   * kept in here. The contract's rule is that a row a person presses and gets
   * nothing from is worse than one that is absent, and a dimmed row carrying
   * the reason is neither.
   */
  dormant?: string;
  /**
   * What a Job, a pull request, an issue or a task says beside its label. **Not stuffed into the label**: the
   * label is what is searched for and marked.
   */
  meta?: PaletteMeta;
  /** What the row is, drawn flush right in a column of its own: Job, Pull request, Page. */
  kind?: string;
};

export type PaletteSection = {
  id: string;
  /**
   * The head. **The context block is titled with the job it acts on**, so a
   * palette that will Kill something says which something before the first row
   * is read.
   */
  title: string;
};

export type CommandPaletteProps = {
  open: boolean;
  /**
   * The sections, in the order the contract fixes: actions available on the
   * current context, navigation, jobs by id or name, settings. **The caller
   * owns the order** — the context block's title changes with the job, so the
   * list cannot be a constant in here.
   */
  sections: readonly PaletteSection[];
  entries: readonly PaletteEntry[];
  /**
   * What was searched, as a sentence, for the state where nothing matched.
   * "every job on this Manifest, at every status" — the host knows the extent
   * of its own index and this component does not.
   */
  searched: string;
  placeholder?: string;
  /** Storybook draws narrowed states, so the query can be seeded from outside. */
  defaultQuery?: string;
  onSelect?: (entry: PaletteEntry) => void;
  onConfirm?: (entry: PaletteEntry) => void;
  onClose?: () => void;
  /** The title bar's one bar: what was typed can be dispatched as well as searched. Absent draws no dispatch row. */
  dispatch?: PaletteDispatch;
  /** A typed path offered as a file to open, first in the list. Absent offers none. */
  path?: PalettePath;
  /** A typed link to a pull request or an issue, offered as the page to open, first in the list. Absent offers none. */
  link?: PaletteLink;
  /**
   * A selector for the box the palette's field opens over, so the search and the field it opened from
   * read as one bar. Absent, or not on screen (the narrow title row hides it), the palette centres.
   */
  anchor?: string;
  /**
   * What the palette shows with nothing typed, in place of the full list: Pinned, then Recent. Absent, the empty
   * query shows every row, which is what the setting "Everything" asks for.
   */
  home?: { sections: readonly PaletteSection[]; entries: readonly PaletteEntry[] };
  /**
   * What ⌘K opens on a row: its acts, short. An empty list is no panel, and the foot says `⌘K Actions` only
   * where the row under the cursor has some.
   */
  actionsOf?: (entry: PaletteEntry) => readonly PaletteAction[];
  /** An action was chosen. A destructive one goes to `onConfirm` instead, as an entry whose id is the row's, a bar and the action's. */
  onAct?: (entry: PaletteEntry, action: PaletteAction) => void;
  /** A row that is not done at once but asks for something first: the second step listing what it can be filled with. */
  stepOf?: (entry: PaletteEntry) => PaletteStep | undefined;
};

/** Whether the query matches, and where in the label it matched. */
type Hit = {
  entry: PaletteEntry;
  /** Where the query sits in the label, or `null` on an alias-only hit. */
  at: number | null;
};

function hit(entry: PaletteEntry, query: string): Hit | null {
  if (query === "") return { entry, at: null };
  const wanted = query.toLowerCase();
  const label = entry.label.toLowerCase();
  const aliases = [...(entry.aliases ?? []), ...(entry.meta === undefined ? [] : [entry.meta.number, entry.meta.repo].flatMap((one) => one ?? []))].map((held) => held.toLowerCase());
  const at = label.indexOf(wanted);
  if (at >= 0) return { entry, at };
  if (aliases.some((held) => held.includes(wanted))) return { entry, at: null };
  // **Words, in any order.** "fix palette" finds "Palette search fix": every word has to begin a word
  // of the label or an alias, and nothing is marked, since no one span matched. Beginning one, because
  // "expand and" must not find "expand the column" on the "and" inside "expand".
  const words = wanted.split(/\s+/).filter((word) => word !== "");
  const starts = [label, ...aliases].flatMap((held) => held.split(/[^a-z0-9]+/));
  const every = words.length > 1 && words.every((word) => starts.some((start) => start.startsWith(word)));
  return every ? { entry, at: null } : null;
}

export function CommandPalette({
  open,
  sections,
  entries,
  searched,
  placeholder = "Search actions, jobs and settings",
  defaultQuery = "",
  onSelect,
  onConfirm,
  onClose,
  dispatch,
  path,
  link,
  anchor,
  home,
  actionsOf,
  onAct,
  stepOf,
}: CommandPaletteProps) {
  const [query, setQuery] = useState(defaultQuery);
  const [at, setAt] = useState(0);
  // Why the path typed was not opened. Gone with the next keystroke, since the words it names are gone.
  const [refused, setRefused] = useState<string | null>(null);
  const input = useRef<HTMLInputElement>(null);
  const active = useRef<HTMLDivElement>(null);
  const panel = useRef<HTMLDivElement>(null);
  // The words a kind is filing and the step listing where they can go, while that step is up.
  const [stepping, setStepping] = useState<{ words: string; step: PaletteStep } | null>(null);
  // The row whose acts are up, and the one of them under the cursor.
  const [acting, setActing] = useState<{ entry: PaletteEntry; at: number } | null>(null);
  const actionRow = useRef<HTMLDivElement>(null);
  // What is typed in the actions panel's own field, which narrows its acts.
  const [actionQuery, setActionQuery] = useState("");
  const bare = query.trim() === "";
  const listed = stepping !== null ? stepping.step : home !== undefined && bare ? home : { entries, sections };

  // Grouped by section in the caller's order, and in the caller's order inside
  // each. Nothing is ranked: the contract fixes what comes first, and a
  // relevance score would reorder a list a person is learning positions in.
  const grouped = useMemo(() => {
    const found = listed.entries.flatMap((entry) => hit(entry, query) ?? []);
    return listed.sections
      .map((section) => ({
        section,
        hits: found.filter((held) => held.entry.section === section.id),
      }))
      .filter((group) => group.hits.length > 0);
  }, [listed, query]);

  const hits = useMemo(() => grouped.flatMap((group) => group.hits), [grouped]);
  // The dispatch row, once something is typed: first where nothing else matches or the words are a
  // link, so ↵ sends them; last otherwise, so ↵ still opens what was searched
  // for. ⌘↵ always sends. A kind that adds to his projects puts it first whatever matches: Tab chose it.
  const offered = dispatch !== undefined && query.trim() !== "" && stepping === null;
  const leading = offered && (hits.length === 0 || readsAsWork(query) || adds(dispatch.kind));
  const typedPath = path === undefined || stepping !== null ? null : readsAsPath(query);
  const rest: (Hit | "dispatch")[] = !offered ? hits : leading ? ["dispatch", ...hits] : [...hits, "dispatch"];
  const typedKind = link === undefined || stepping !== null ? null : link.kindOf(query.trim());
  const flat: (Hit | "dispatch" | "path" | "link")[] = [...(typedKind === null ? [] : ["link" as const]), ...(typedPath === null ? [] : ["path" as const]), ...rest];
  const place = usePlace(open, anchor, panel);
  // The row under the cursor and what ⌘K would open on it.
  const under = flat[at];
  const underEntry = typeof under === "object" ? under.entry : undefined;
  const underActions = underEntry === undefined || actionsOf === undefined || stepping !== null ? [] : actionsOf(underEntry);

  // **The first row is always active**, so a query narrowed to one result is a
  // two-key act. Reset on every change to the query rather than clamped: a
  // cursor left at position four in a list that now has two is a cursor
  // somewhere the person did not put it.
  useEffect(() => (setAt(0), setRefused(null)), [query]);
  // Closed, it forgets what was typed: the one bar reopens empty rather than holding words `n` could send.
  useEffect(() => void (open || (setQuery(defaultQuery), setStepping(null), setActing(null))), [open, defaultQuery]);
  // Tab to a kind that adds makes its row the active one, so Enter adds.
  const kind = dispatch?.kind;
  useEffect(() => {
    if (kind !== undefined && adds(kind)) setAt(typedPath === null ? 0 : 1);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [kind]);

  useEffect(() => {
    if (open) input.current?.focus();
  }, [open]);

  // Esc closes the palette and stops there, wherever focus went. On `window` in the capture phase
  // for the reason `Popover` is: a field-only handler goes deaf once a click blurs the field, and
  // a Job detail or sheet listening on the same window would answer the same press.
  useEffect(() => {
    if (!open) return;
    function onEscape(event: KeyboardEvent) {
      if (event.key !== "Escape" || event.defaultPrevented) return;
      // A confirmation opened over the palette is the top layer, and the press is its to answer.
      const layer = (event.target as Element | null)?.closest?.('[role="dialog"]');
      if (layer && !layer.classList.contains("armada-palette")) return;
      event.preventDefault();
      event.stopPropagation();
      // The acts back out to the results, and the query stays.
      if (acting !== null) {
        setActing(null);
        return;
      }
      // A second step backs out to the words it was filing.
      if (stepping !== null) {
        setQuery(stepping.words);
        setStepping(null);
        return;
      }
      onClose?.();
    }
    window.addEventListener("keydown", onEscape, true);
    return () => window.removeEventListener("keydown", onEscape, true);
  }, [open, onClose, stepping, acting]);

  // ⌘K opens the acts of the row under the cursor, and closes them again. The press is marked handled so
  // the binding that toggles the palette leaves it up; on a row with no acts it is not, and closes it as ever.
  useEffect(() => {
    if (!open) return;
    function onActs(event: KeyboardEvent) {
      if (event.defaultPrevented || !isPressed("command_palette", event)) return;
      if (acting !== null) {
        event.preventDefault();
        setActing(null);
        return;
      }
      if (underEntry === undefined || underActions.length === 0) return;
      event.preventDefault();
      setActing({ entry: underEntry, at: 0 });
    }
    window.addEventListener("keydown", onActs, true);
    return () => window.removeEventListener("keydown", onActs, true);
  }, [open, acting, underEntry, underActions.length]);

  // The panel's field starts empty on every opening, and the palette's own field takes focus back when it goes.
  useEffect(() => {
    setActionQuery("");
    if (acting === null && open) input.current?.focus();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [acting === null, acting?.entry.id]);

  // An act's own chord runs it on the row under the cursor, from the results or from the panel, as Raycast's do.
  useEffect(() => {
    if (!open || stepping !== null) return;
    const entry = acting?.entry ?? underEntry;
    if (entry === undefined) return;
    const acts = actionsOf?.(entry) ?? [];
    function onChord(event: KeyboardEvent) {
      if (event.defaultPrevented || entry === undefined) return;
      const action = acts.find((one) => one.shortcut !== undefined && pressedChord(one.shortcut, event));
      if (action === undefined) return;
      event.preventDefault();
      event.stopPropagation();
      runAction(entry, action);
    }
    window.addEventListener("keydown", onChord, true);
    return () => window.removeEventListener("keydown", onChord, true);
  });

  // A click outside `.armada-palette` closes it, following `Popover`'s `contains(target)` check —
  // not `event.target === layer`, which breaks the moment anything else nests in the layer.
  useEffect(() => {
    if (!open) return;
    function onDown(event: MouseEvent) {
      const target = event.target as Element | null;
      // A confirmation opened over the palette is the top layer, and the click is its to answer.
      const dialog = target?.closest?.('[role="dialog"]');
      if (dialog && !dialog.classList.contains("armada-palette")) return;
      if (panel.current?.contains(target as Node | null)) return;
      onClose?.();
    }
    window.addEventListener("mousedown", onDown);
    return () => window.removeEventListener("mousedown", onDown);
  }, [open, onClose]);

  // The active row is kept in view. `nearest` and not `center`: the list is
  // scrolled past 400px and centering would jump a row that was already
  // visible, which is the moving target the top anchor exists to prevent.
  useEffect(() => {
    active.current?.scrollIntoView({ block: "nearest" });
  }, [at]);

  if (!open) return null;

  function send() {
    if (dispatch === undefined || query.trim() === "" || stepping !== null) return;
    const words = query.trim();
    const step = dispatch.step?.(words, dispatch.kind);
    if (step !== undefined) {
      setStepping({ words, step });
      setQuery("");
      return;
    }
    dispatch.onDispatch(words, dispatch.kind);
    onClose?.();
  }

  function openPath() {
    if (path === undefined || typedPath === null) return;
    void Promise.resolve(path.onOpen(typedPath)).then((opened) => {
      if (opened) onClose?.();
      else setRefused(`Could not open “${typedPath}”. It is not a readable file here.`);
    });
  }

  function openLink() {
    if (link === undefined || typedKind === null) return;
    void Promise.resolve(link.onOpen(query.trim(), typedKind)).then((opened) => {
      if (opened) onClose?.();
      else setRefused(`Could not open “${query.trim()}”.`);
    });
  }

  function choose(held: Hit | "dispatch" | "path" | "link" | undefined) {
    if (held === undefined) return;
    if (held === "link") return openLink();
    if (held === "path") return openPath();
    if (held === "dispatch") return send();
    // Drawn so the binding can be learned, and inert because the row already
    // says why. Pressing it does nothing rather than reaching something
    // invented.
    if (held.entry.dormant !== undefined) return;
    if (stepping !== null) {
      stepping.step.onChoose(held.entry);
      onClose?.();
      return;
    }
    const step = stepOf?.(held.entry);
    if (step !== undefined) {
      setStepping({ words: query, step });
      setQuery("");
      return;
    }
    if (held.entry.destructive === true) {
      onConfirm?.(held.entry);
      return;
    }
    onSelect?.(held.entry);
    onClose?.();
  }

  function runAction(entry: PaletteEntry, action: PaletteAction | undefined) {
    if (action === undefined || action.dormant !== undefined) return;
    if (action.destructive === true) {
      onConfirm?.({ ...entry, id: actionEntryId(entry, action), label: action.label, destructive: true });
      return;
    }
    onAct?.(entry, action);
    if (action.stays === true) setActing(null);
    else onClose?.();
  }

  // Only the navigation set is intercepted here, and `Esc` above. Every other
  // key belongs to the query, which is the suppression rule holding.
  function onKey(event: React.KeyboardEvent<HTMLInputElement>) {
    if (acting !== null) return;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setAt((n) => (flat.length === 0 ? 0 : (n + 1) % flat.length));
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      setAt((n) => (flat.length === 0 ? 0 : (n - 1 + flat.length) % flat.length));
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      if (event.metaKey) return send();
      choose(flat[at]);
      return;
    }
    if (event.key === "Tab" && dispatch?.onKind !== undefined && stepping === null && !event.altKey && !event.ctrlKey && !event.metaKey) {
      event.preventDefault();
      dispatch.onKind(nextKind(dispatch));
    }
  }

  // The panel's field: ↑ ↓ move through the acts it narrows to, ↵ runs one. Esc and ⌘K are the window's, above.
  function onPanelKey(event: React.KeyboardEvent<HTMLInputElement>) {
    if (acting === null) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const step = event.key === "ArrowDown" ? 1 : -1;
      setActing({ ...acting, at: actionList.length === 0 ? 0 : (acting.at + step + actionList.length) % actionList.length });
    } else if (event.key === "Enter") {
      event.preventDefault();
      runAction(acting.entry, actionList[acting.at]);
    } else if (event.key === "Tab") {
      event.preventDefault();
    }
  }

  let n = (leading ? 0 : -1) + (typedPath === null ? 0 : 1) + (typedKind === null ? 0 : 1);
  const current = flat[at];
  const actionList = acting === null ? [] : narrowed(actionsOf?.(acting.entry) ?? [], actionQuery);
  const sendRow = !offered ? null : (
    <DispatchRow words={query.trim()} kind={dispatch.kind} trailing={!leading} active={current === "dispatch"} ref={current === "dispatch" ? active : undefined} onEnter={() => setAt(flat.indexOf("dispatch"))} onChoose={send} />
  );

  return (
    <div className="armada-palette-layer" data-anchored={place.style === undefined ? undefined : ""}>
      {/* The window's middle lines, drawn only while the palette is dragged onto one. */}
      {place.snapped?.across === true ? <div className="armada-palette__guide armada-palette__guide--across" aria-hidden /> : null}
      {place.snapped?.down === true ? <div className="armada-palette__guide armada-palette__guide--down" aria-hidden /> : null}
      <div
        ref={panel}
        className="armada-palette"
        role="dialog"
        aria-modal="true"
        aria-label="Command palette"
        data-acting={acting === null ? undefined : ""}
        data-moving={place.snapped === null ? undefined : ""}
        style={place.style}
        onPointerDown={place.onPointerDown}
        onDoubleClick={place.onDoubleClick}
      >
        <input
          ref={input}
          className="armada-palette__input"
          type="text"
          role="combobox"
          aria-expanded="true"
          aria-controls="armada-palette-list"
          aria-activedescendant={acting !== null || current === undefined ? undefined : current === "link" ? LINK_ROW : current === "path" ? PATH_ROW : current === "dispatch" ? DISPATCH_ROW : rowId(current.entry)}
          aria-label={stepping?.step.title ?? placeholder}
          placeholder={stepping === null ? placeholder : ""}
          value={query}
          readOnly={acting !== null}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={onKey}
        />
        <div className="armada-palette__list" id="armada-palette-list" role="listbox">
          {stepping === null ? null : <p className="armada-palette__step">{stepping.step.title}</p>}
          {typedKind === null ? null : <LinkRow kind={typedKind} link={query.trim()} active={current === "link"} ref={current === "link" ? active : undefined} onEnter={() => setAt(0)} onChoose={openLink} />}
          {typedPath === null ? null : <PathRow path={typedPath} active={current === "path"} ref={current === "path" ? active : undefined} onEnter={() => setAt(typedKind === null ? 0 : 1)} onChoose={openPath} />}
          {refused === null ? null : <p className="armada-palette__empty" role="alert">{refused}</p>}
          {leading ? sendRow : null}
          {hits.length === 0 && typedPath === null && typedKind === null ? (
            /* **Names the query and says what was searched.** No suggestions
               and no did-you-mean: the palette is the discovery surface, so
               the honest answer to a miss is the extent of the index. */
            <p className="armada-palette__empty">
              {`Nothing matches “${query}”. Searched ${stepping?.step.searched ?? searched}.`}
            </p>
          ) : null}
          {grouped.map((group) => (
            <div key={group.section.id} className="armada-palette__group">
              {/* Section heads are what stop four kinds of result reading as
                  one list. Drawn only where the section has a hit. */}
              <div className="armada-palette__section">{group.section.title}</div>
              {group.hits.map((held) => {
                n += 1;
                const where = n;
                return (
                  <Row
                    key={held.entry.id}
                    hit={held}
                    query={query}
                    active={where === at}
                    ref={where === at ? active : undefined}
                    onEnter={() => setAt(where)}
                    onChoose={() => choose(held)}
                  />
                );
              })}
            </div>
          ))}
          {offered && !leading ? sendRow : null}
        </div>
        {acting === null ? null : (
          <ActionsPanel
            entry={acting.entry}
            actions={actionList}
            at={acting.at}
            query={actionQuery}
            active={actionRow}
            onQuery={(typed) => (setActionQuery(typed), setActing({ ...acting, at: 0 }))}
            onKey={onPanelKey}
            onEnter={(next) => setActing({ ...acting, at: next })}
            onChoose={(action) => runAction(acting.entry, action)}
          />
        )}
        {acting !== null ? (
          <ResultKeys />
        ) : offered && dispatch !== undefined ? (
          <DispatchKeys dispatch={dispatch} sends={current === "dispatch"} acts={underActions.length > 0} />
        ) : stepping !== null ? (
          <StepKeys />
        ) : underActions.length > 0 ? (
          <ResultKeys />
        ) : null}
      </div>
    </div>
  );
}

/** The dom id of a row, so `aria-activedescendant` can name it. */
function rowId(entry: PaletteEntry): string {
  return `armada-palette-row-${entry.id}`;
}

function Row({
  hit: held,
  query,
  active,
  ref,
  onEnter,
  onChoose,
}: {
  hit: Hit;
  query: string;
  active: boolean;
  ref?: React.Ref<HTMLDivElement>;
  onEnter: () => void;
  onChoose: () => void;
}) {
  const { entry } = held;
  const Glyph = entry.icon;
  const dormant = entry.dormant !== undefined;
  const classes = [
    "armada-palette__row",
    active ? "armada-palette__row--active" : "",
    entry.destructive === true ? "armada-palette__row--danger" : "",
    dormant ? "armada-palette__row--dormant" : "",
  ]
    .filter((held) => held !== "")
    .join(" ");

  return (
    // A row is an option and not a button: focus stays in the query field, so
    // typing carries on while the cursor moves, and `aria-activedescendant` on
    // the field is what says which row the cursor is on.
    <div
      ref={ref}
      id={rowId(entry)}
      role="option"
      aria-selected={active}
      aria-disabled={dormant ? true : undefined}
      className={classes}
      onMouseEnter={onEnter}
      onClick={onChoose}
    >
      {/* The slot holds its width whether or not there is a glyph, so a
          section with two icons and six blanks still reads as a column. A
          Job's state is its glyph. */}
      <span className="armada-palette__glyph">
        {entry.meta?.state !== undefined ? <StateGlyph state={entry.meta.state} /> : Glyph === undefined ? null : <Glyph size={16} strokeWidth={2} aria-hidden />}
      </span>
      <span className="armada-palette__label">
        <span className="armada-palette__title">
          <Marked label={entry.label} query={query} at={held.at} />
        </span>
        <Subtitle meta={entry.meta} value={entry.value} dormant={entry.dormant} />
      </span>
      {/* Two fixed columns, so every kind and every binding ends on the same
          edge as the rows above it: the kind, then the key at the far right. */}
      <span className="armada-palette__kind">{entry.kind}</span>
      <span className="armada-palette__keys-col">{entry.shortcut === undefined ? null : <Shortcut shortcut={entry.shortcut} />}</span>
    </div>
  );
}

/**
 * The label, with the matched span marked.
 *
 * **An alias hit marks nothing.** `at` is `null` there, because the match was
 * on a word that never renders — no "matched terminate", no highlight, and
 * nothing faked onto the lexicon term standing in its place.
 *
 * The mark is contrast and not weight or hue: the rest of the label steps down
 * to `--fg-muted` and the span keeps `--fg-default`. Status hue is never chosen
 * and the accent is spoken for. A danger row underlines the span instead,
 * because red has no contrast step.
 */
function Marked({ label, query, at }: { label: string; query: string; at: number | null }) {
  if (at === null || query === "") return <>{label}</>;
  return (
    <>
      {label.slice(0, at)}
      <mark className="armada-palette__mark">{label.slice(at, at + query.length)}</mark>
      {label.slice(at + query.length)}
    </>
  );
}

/**
 * The binding, cut into one box per chord and a separator between two.
 *
 * **The registry's spelling is the input and nothing is re-spelled here.**
 * `⌘K`, `j / k / ↓ / ↑`, `⌘1–⌘6` and `[ ]` are all how `actions.toml` writes
 * them, and the gate holds that file to the contract's map — so the palette
 * shows exactly the string a person will read in the contract. What this does
 * is put every chord — the keys of one binding held together — in its own
 * box; `/` and `–` still separate two distinct bindings and are drawn as
 * punctuation between boxes, never folded into one.
 */
function Shortcut({ shortcut }: { shortcut: string }) {
  return (
    <span className="armada-palette__keys">
      {piecesOf(shortcut).map((piece, i) =>
        piece.chord === undefined ? (
          <span key={i} className="armada-palette__between">
            {piece.between}
          </span>
        ) : (
          <KbdChord key={i} keys={piece.chord} />
        ),
      )}
    </span>
  );
}

/** A chord that gets one box, or a separator drawn between two. */
type Piece = { chord: string[]; between?: undefined } | { chord?: undefined; between: string };

/**
 * Cut a binding into chords and separators.
 *
 * A space separates two whole bindings and needs no mark — the gap between
 * boxes is the separation. `/` and `–` are kept and drawn, because `j / k`
 * means either key and `⌘1–⌘6` means a range: both are two distinct bindings,
 * never one chord, so the separator stays outside every box. A leading `⌘`
 * joins the box it already started rather than starting one of its own — the
 * combination `⌘K` is one press, and one box is what reads as one.
 */
function piecesOf(shortcut: string): Piece[] {
  return shortcut
    .split(/(\s+|\/|–)/)
    .flatMap((part): Piece[] => {
      if (part.trim() === "") return [];
      if (part === "/" || part === "–") return [{ between: part }];
      if (part.length > 1 && part.startsWith("⌘")) {
        return [{ chord: ["⌘", part.slice(1)] }];
      }
      return [{ chord: [part] }];
    });
}
