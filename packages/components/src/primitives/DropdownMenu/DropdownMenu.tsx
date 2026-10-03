import { Check, ChevronDown } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useEffect, useLayoutEffect, useRef, useState } from "react";

import { Button } from "../Button/Button";

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
 * No glyphs otherwise: icons stay on ghost/icon-only row actions, confirmation
 * dialogs and toolbars, per iconography; one drawing in the sheet disagrees,
 * see the report.
 */
export type DropdownMenuEntry =
  | {
      kind: "item";
      id: string;
      label: string;
      shortcut?: string;
      danger?: boolean;
      selected?: boolean;
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
   * the menu-trigger mark's own row in `packages/icons/icons.toml` keeps it off
   * a trigger with no label, and `aria-haspopup` says it opens a menu.
   */
  icon?: LucideIcon;
};

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

  // Esc closes an overlay, per the global tier.
  useEffect(() => {
    if (!open) return;
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape") close.current(false);
    }
    function onDown(event: MouseEvent) {
      if (!root.current?.contains(event.target as Node)) close.current(false);
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
      {Glyph === undefined ? (
        <button
          ref={trigger}
          type="button"
          className="armada-dropdown-menu__trigger"
          aria-haspopup="menu"
          aria-expanded={open && !disabled}
          disabled={disabled}
          onClick={() => setOpen(!open)}
        >
          {triggerLabel}
          <Count of={triggerCount} />
          {/* Says this opens a menu, settled 2026-09-17 (icons.toml, chevron-down).
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
      {shown ? (
        <div ref={panel} className="armada-dropdown-menu__panel" role="menu">
          {entries.map((entry) => {
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
            return (
              <button
                key={entry.id}
                type="button"
                role="menuitem"
                aria-current={entry.selected || undefined}
                className={
                  entry.danger
                    ? "armada-dropdown-menu__item armada-dropdown-menu__item--danger"
                    : "armada-dropdown-menu__item"
                }
                onClick={() => {
                  setOpen(false);
                  onSelect?.(entry.id);
                }}
              >
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
          })}
        </div>
      ) : null}
    </div>
  );
}

function Count({ of }: { of: number | undefined }) {
  return of === undefined || of === 0 ? null : (
    <span className="armada-dropdown-menu__count">{of}</span>
  );
}
