import { Check, ChevronDown, X } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type ReactNode } from "react";
import { createPortal } from "react-dom";

import { Button } from "../Button/Button";
import { Input } from "../Input/Input";
import { Tooltip } from "../Tooltip/Tooltip";

/**
 * The row menu, and the one place besides a dialog, sheet, popover, tooltip
 * and the palette where a shadow is legal. `--bg-overlay`, `--border-default`,
 * `--radius-lg`, and no blur.
 *
 * Items carry a right-aligned kbd where the action has a binding, and item
 * height is unchanged by it. The destructive item sits last and below a
 * separator. `selected` takes that same slot as a checkmark, and
 * `aria-current` carries the fact for anyone not reading the glyph — a chosen
 * item never also has a shortcut.
 *
 * No glyphs by default: icons stay on ghost/icon-only row actions, confirmation
 * dialogs and toolbars, per iconography. **An item may carry its act's glyph**
 * (`icon`), leading its label, for a menu that gathers a header's acts (the
 * owner, 16 Oct 2026): the same glyph the act wore as a button.
 */
export type DropdownMenuEntry =
  | {
      kind: "item";
      id: string;
      label: string;
      shortcut?: string;
      danger?: boolean;
      /** The act's glyph, before the label. */
      icon?: LucideIcon;
      selected?: boolean;
      /**
       * Listed and not choosable: `aria-disabled`, `--fg-subtle`, and a press
       * does nothing. It stays in the arrow order so a person can read why.
       */
      disabled?: boolean;
      /** What a hover on the item says: why a disabled one is not choosable, or what a plain one starts. */
      hint?: string;
      /**
       * How many rows the item holds, where the menu is a panel's filters (the
       * owner, 29 Sep 2026). Tabs' count rule: trailing mono. Zero draws nothing.
       */
      count?: number;
    }
  | { kind: "separator"; id: string }
  | { kind: "label"; id: string; label: string };

export type DropdownMenuProps = {
  /** Sentence case, and it names what the menu is for. */
  triggerLabel: string;
  /** How many the trigger's chosen filter holds. Zero draws nothing. */
  triggerCount?: number;
  /**
   * Which of the trigger's edges the menu lines up with. Trailing by default,
   * per Floating layers; `start` for a trigger that leads its row, whose menu
   * would otherwise hang out past the panel it sits in.
   */
  align?: "start" | "end";
  entries: DropdownMenuEntry[];
  defaultOpen?: boolean;
  /**
   * Held by the caller, for a menu something besides its trigger opens — a
   * Studio's Run, which `R` and the palette open as a press on it does (the
   * owner, 2 Oct 2026). Absent, the menu holds its own. Every close the menu
   * makes itself is reported to `onOpenChange`.
   */
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  /**
   * The trigger is off and the menu does not open — for a menu whose every
   * item sends something, while a send is already out. Disabled is
   * `--fg-subtle` text with hover suppressed, never an opacity, which is the
   * global rule the contract gives every control.
   */
  disabled?: boolean;
  onSelect?: (id: string) => void;
  /**
   * Draw the trigger as an icon-only ghost button named by `triggerLabel`, for
   * a canvas rail's act — Run, the owner's call of 2 Oct 2026. **No chevron**:
   * the menu-trigger mark's own row in `packages/icons/icons/` keeps it off
   * a trigger with no label, and `aria-haspopup` says it opens a menu.
   */
  icon?: LucideIcon;
  /**
   * Draw the panel in a layer of its own on the page, over everything, placed
   * from the trigger's box. For a menu whose trigger sits inside something
   * that makes its own stacking context — a node on a canvas — where the panel
   * would otherwise open behind the next node. **An element** draws the panel in that element instead of
   * the page's body, for a trigger inside a floating window whose own layer sits over the body's.
   */
  portal?: boolean | HTMLElement;
  /**
   * A filter field at the head of the panel, named by this. For a menu long
   * enough that finding a row by eye is the cost — the repository picker, at
   * twenty repositories. Typing narrows the items by their label, a label or
   * rule with nothing left under it goes with them, Enter takes the first
   * choosable row left, and ↓ walks into the list.
   */
  searchLabel?: string;
  /**
   * The field's text held by the caller, for a menu whose search narrows something besides
   * its own items (the Cockpit's list). Typing reports through `onChange`, the items are not
   * filtered by it, and Enter closes the menu and reports through `onEnter`. Needs `searchLabel`.
   */
  search?: { value: string; onChange: (value: string) => void; onEnter: () => void };
  /**
   * The trigger as plain text and its chevron, with no box, for a menu that stands in a panel's
   * header band (the owner, 16 Oct 2026).
   */
  plain?: boolean;
  /**
   * What the trigger is called to a screen reader where its text says only the current choice —
   * Helm's repository switch shows the repository it stands on, and the name says what pressing
   * it does. Absent, the trigger is named by its text.
   */
  triggerAriaLabel?: string;
  /**
   * A point in the window the panel opens at, for a context menu: no trigger is drawn, the panel is portalled and its
   * leading top corner sits on the point, kept inside the window. Open it through `open`.
   */
  at?: { x: number; y: number };
};

/**
 * What a typed query leaves of a menu's entries.
 *
 * **A heading with nothing under it is dropped.** A group whose rows all fell
 * out would otherwise stand as a label over nothing. Where the query is blank
 * every entry stays, as it was.
 */
export function filterEntries(entries: DropdownMenuEntry[], query: string): DropdownMenuEntry[] {
  const needle = query.trim().toLowerCase();
  if (needle === "") return entries;
  const kept: DropdownMenuEntry[] = [];
  let heading: DropdownMenuEntry[] = [];
  for (const entry of entries) {
    if (entry.kind !== "item") {
      heading = entry.kind === "label" ? [entry] : [];
      continue;
    }
    if (!entry.label.toLowerCase().includes(needle)) continue;
    kept.push(...heading, entry);
    heading = [];
  }
  return kept;
}

export function DropdownMenu({
  triggerLabel,
  triggerCount,
  align = "end",
  entries,
  defaultOpen = false,
  open: held,
  onOpenChange,
  disabled = false,
  onSelect,
  icon: Glyph,
  portal = false,
  searchLabel,
  search: field,
  plain = false,
  triggerAriaLabel,
  at,
}: DropdownMenuProps) {
  const [own, setOwn] = useState(defaultOpen);
  const open = held ?? own;
  const setOpen = (next: boolean) => {
    if (held === undefined) setOwn(next);
    onOpenChange?.(next);
  };
  // The listeners below are added once per opening, so they read the latest
  // close through this rather than the one in force when they were added.
  const close = useRef(setOpen);
  close.current = setOpen;
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const panel = useRef<HTMLDivElement>(null);
  const shown = open && !disabled;
  const [typed, setQuery] = useState("");
  const query = field?.value ?? typed;
  const search = useRef<HTMLInputElement>(null);
  const visible = searchLabel === undefined || field !== undefined ? entries : filterEntries(entries, query);
  const choosable = visible.flatMap((entry) => (entry.kind === "item" && entry.disabled !== true ? [entry] : []));

  // A reopened menu starts unfiltered: the last query is not what its next
  // opening is for.
  useEffect(() => {
    if (!shown) setQuery("");
  }, [shown]);

  // Which side the menu resolved to, read before the first paint so its
  // entrance rises from that side. Anchor positioning chooses a fallback in
  // layout and no selector can see which, so the boxes are compared instead.
  // Written straight onto the element rather than into state: it is a fact
  // about this one layout, and a render for it would be a second commit
  // between the layout and the paint. The panel is unmounted on close, so a
  // reopen measures afresh.
  useLayoutEffect(() => {
    const layer = panel.current;
    const anchor = trigger.current;
    if (!shown || layer === null || anchor === null) return;
    const at = layer.getBoundingClientRect();
    const from = anchor.getBoundingClientRect();
    layer.dataset.opens = at.top < from.top ? "up" : "down";
    layer.dataset.aligns = Math.abs(at.right - from.right) <= Math.abs(at.left - from.left) ? "end" : "start";
  }, [shown]);

  // A portalled panel has no anchor to take its place from, so it is set from the trigger's box:
  // below it with the trailing edges flush, above it where there is no room below.
  useLayoutEffect(() => {
    const layer = panel.current;
    const anchor = trigger.current;
    if (at !== undefined && shown && layer !== null) {
      const gap = 4;
      const size = layer.getBoundingClientRect();
      layer.style.left = `${Math.max(gap, Math.min(at.x, window.innerWidth - size.width - gap))}px`;
      layer.style.top = `${Math.max(gap, Math.min(at.y, window.innerHeight - size.height - gap))}px`;
      return;
    }
    if (!portal || !shown || layer === null || anchor === null) return;
    const from = anchor.getBoundingClientRect();
    const own = layer.getBoundingClientRect();
    const gap = 4;
    // Leading edges flush where the caller asked for them, as the unportalled panel is.
    const flush = align === "start" ? from.left : from.right - own.width;
    const left = Math.max(gap, Math.min(flush, window.innerWidth - own.width - gap));
    const below = from.bottom + gap;
    layer.style.left = `${left}px`;
    layer.style.top = `${below + own.height > window.innerHeight ? Math.max(gap, from.top - gap - own.height) : below}px`;
  }, [shown, portal, at]);

  const layered = (node: ReactNode): ReactNode => (portal || at !== undefined ? createPortal(node, typeof portal === "object" ? portal : document.body) : node);

  // **Focus goes into the menu as it opens**, onto its first item, so the
  // arrows and Enter work however it was opened — a press, or a key that is not
  // the trigger's, which left focus where it was (Run's `R`, the owner, 2 Oct
  // 2026).
  useEffect(() => {
    if (!shown) return;
    if (searchLabel !== undefined) search.current?.focus();
    else itemsOf(panel.current)[0]?.focus();
  }, [shown, searchLabel]);

  // Esc closes an overlay, per the global tier, and hands focus back to the
  // trigger where it was in the menu, rather than dropping it on the page.
  useEffect(() => {
    if (!open) return;
    function onKey(event: KeyboardEvent) {
      if (event.key !== "Escape") return;
      if (panel.current?.contains(document.activeElement) === true) trigger.current?.focus();
      close.current(false);
    }
    function onDown(event: MouseEvent) {
      const hit = event.target as Node;
      if (!root.current?.contains(hit) && panel.current?.contains(hit) !== true) close.current(false);
    }
    window.addEventListener("keydown", onKey);
    window.addEventListener("mousedown", onDown);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("mousedown", onDown);
    };
  }, [open]);

  return (
    <div
      className="armada-dropdown-menu"
      ref={root}
      data-align={align === "start" ? "start" : undefined}
    >
      {at !== undefined ? null : Glyph === undefined ? (
        <button
          ref={trigger}
          type="button"
          className="armada-dropdown-menu__trigger"
          data-plain={plain || undefined}
          aria-label={triggerAriaLabel}
          aria-haspopup="menu"
          aria-expanded={open && !disabled}
          disabled={disabled}
          onClick={() => setOpen(!open)}
        >
          {triggerLabel}
          <Count of={triggerCount} />
          {/* Says this opens a menu, settled 2026-09-17 (icons/, chevron-down).
              Every labelled trigger carries it. */}
          <ChevronDown className="armada-dropdown-menu__chevron" size={12} strokeWidth={2} aria-hidden />
        </button>
      ) : (
        <Button
          ref={trigger}
          variant="ghost"
          size="sm"
          iconOnly
          aria-label={triggerLabel}
          aria-haspopup="menu"
          aria-expanded={open && !disabled}
          disabled={disabled}
          onClick={() => setOpen(!open)}
        >
          <Glyph size={16} strokeWidth={2} aria-hidden />
        </Button>
      )}
      {/* A menu open when its trigger turns off stays shut rather than sending
          from under a control that says it cannot. */}
      {shown ? layered(
        <div ref={panel} className="armada-dropdown-menu__panel" data-portal={portal !== false || at !== undefined || undefined} data-searchable={searchLabel !== undefined || undefined} role="menu" onKeyDown={moveFocus}>
          {searchLabel === undefined ? null : (
            <div className="armada-dropdown-menu__search">
              <Input
                ref={search}
                role="searchbox"
                aria-label={searchLabel}
                autoComplete="off"
                value={query}
                onChange={(event) => (field?.onChange ?? setQuery)(event.target.value)}
                trailing={
                  query === "" ? undefined : (
                    <Tooltip label="Clear search">
                      <button
                        type="button"
                        className="armada-dropdown-menu__clear"
                        aria-label="Clear search"
                        onClick={() => {
                          (field?.onChange ?? setQuery)("");
                          search.current?.focus();
                        }}
                      >
                        <X size={14} strokeWidth={2} aria-hidden />
                      </button>
                    </Tooltip>
                  )
                }
                onKeyDown={(event) => {
                  if (event.key !== "Enter") return;
                  event.preventDefault();
                  if (field !== undefined) {
                    setOpen(false);
                    field.onEnter();
                    return;
                  }
                  const first = choosable[0];
                  if (first === undefined) return;
                  setOpen(false);
                  onSelect?.(first.id);
                }}
              />
            </div>
          )}
          {searchLabel !== undefined && visible.length === 0 ? (
            <p className="armada-dropdown-menu__empty" role="status">{`Nothing matches “${query.trim()}”.`}</p>
          ) : null}
          {visible.map((entry) => {
            if (entry.kind === "separator") {
              return <div key={entry.id} className="armada-dropdown-menu__separator" role="separator" />;
            }
            if (entry.kind === "label") {
              return (
                <div key={entry.id} className="armada-dropdown-menu__label">
                  {entry.label}
                </div>
              );
            }
            const item = (
              <button
                key={entry.id}
                type="button"
                role="menuitem"
                aria-current={entry.selected || undefined}
                aria-disabled={entry.disabled || undefined}
                className={
                  entry.danger
                    ? "armada-dropdown-menu__item armada-dropdown-menu__item--danger"
                    : "armada-dropdown-menu__item"
                }
                onClick={() => {
                  if (entry.disabled === true) return;
                  setOpen(false);
                  onSelect?.(entry.id);
                }}
              >
                {entry.icon === undefined ? null : <entry.icon className="armada-dropdown-menu__icon" size={16} strokeWidth={2} aria-hidden />}
                <span className="armada-dropdown-menu__text">
                  {entry.label}
                  <Count of={entry.count} />
                </span>
                {entry.selected ? (
                  <Check className="armada-dropdown-menu__check" size={16} strokeWidth={2} aria-hidden />
                ) : entry.shortcut ? (
                  <kbd className="armada-dropdown-menu__kbd">{entry.shortcut}</kbd>
                ) : null}
              </button>
            );
            return entry.hint !== undefined ? (
              <Tooltip key={entry.id} label={entry.hint} asChild>
                {item}
              </Tooltip>
            ) : (
              item
            );
          })}
        </div>
      ) : null}
    </div>
  );
}

/** A menu's items, in the order it draws them. */
function itemsOf(panel: HTMLElement | null): HTMLElement[] {
  return [...(panel?.querySelectorAll<HTMLElement>('[role="menuitem"]') ?? [])];
}

/** ↑ and ↓ step through the items and wrap; Home and End go to either end. Enter is the item's own. */
function moveFocus(event: ReactKeyboardEvent<HTMLDivElement>): void {
  // In the filter field only ↓ and ↑ are the menu's: Home and End move its caret.
  if ((event.target as HTMLElement).getAttribute("role") === "searchbox" && event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
  const items = itemsOf(event.currentTarget);
  const at = items.indexOf(document.activeElement as HTMLElement);
  const to =
    event.key === "ArrowDown"
      ? (at + 1) % items.length
      : event.key === "ArrowUp"
        ? (at <= 0 ? items.length : at) - 1
        : event.key === "Home"
          ? 0
          : event.key === "End"
            ? items.length - 1
            : null;
  if (to === null || items.length === 0) return;
  event.preventDefault();
  items[to]?.focus();
}

function Count({ of }: { of: number | undefined }) {
  return of === undefined || of === 0 ? null : (
    <span className="armada-dropdown-menu__count">{of}</span>
  );
}
