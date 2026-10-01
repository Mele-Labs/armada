import { useEffect, useRef, useState, type ReactNode } from "react";

/**
 * A floating layer anchored to the control that opened it. The contract names
 * popover among the surfaces that take `--bg-overlay` and among the three
 * where a shadow is legal, and says nothing else about it — no radius, no
 * padding, no width, no use. Those are read off the dropdown-menu line and
 * reported.
 *
 * Never a second floating layer inside it: elevation does not stack.
 */
export type PopoverAlign = "start" | "end";

export type PopoverProps = {
  /** The control that opens it. Renders in the normal flow. */
  trigger: ReactNode;
  /** A function is handed `close`, for a panel whose acts should put it away. */
  children: ReactNode | ((close: () => void) => ReactNode);
  align?: PopoverAlign;
  defaultOpen?: boolean;
  /** Names the layer, where more than one popover can be open on a page in turn. */
  label?: string;
  /**
   * Held by the caller rather than by the trigger. **The trigger then toggles
   * nothing**: a press that opens the layer is the caller's own, which is how a
   * key and a button can open the same one.
   */
  open?: boolean;
  /**
   * Esc, or a press outside, while `open` is the caller's. The caller decides
   * whether it closes — a layer holding unsent writing may refuse a stray press.
   */
  onDismiss?: (how: "escape" | "outside") => void;
};

export function Popover({
  trigger,
  children,
  align = "start",
  defaultOpen = false,
  label,
  open: held,
  onDismiss,
}: PopoverProps) {
  const [own, setOwn] = useState(defaultOpen);
  const controlled = held !== undefined;
  const open = controlled ? held : own;
  const root = useRef<HTMLDivElement>(null);

  // Esc closes an overlay, per the global tier, and stops there. Marked taken as well as stopped:
  // a sheet listening on the same window cannot be stopped, and reads the mark instead.
  useEffect(() => {
    if (!open) return;
    function onKey(event: KeyboardEvent) {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      if (controlled) onDismiss?.("escape");
      else setOwn(false);
    }
    function onDown(event: MouseEvent) {
      if (root.current?.contains(event.target as Node)) return;
      if (controlled) onDismiss?.("outside");
      else setOwn(false);
    }
    window.addEventListener("keydown", onKey, true);
    // Captured, because a canvas's pane stops the press it pans with before it
    // can bubble — and a press on the canvas is outside every layer over it.
    window.addEventListener("mousedown", onDown, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("mousedown", onDown, true);
    };
  }, [open, controlled, onDismiss]);

  return (
    <div className="armada-popover" ref={root}>
      <span className="armada-popover__trigger" onClick={controlled ? undefined : () => setOwn(!own)}>
        {trigger}
      </span>
      {open ? (
        <div
          className={
            align === "end"
              ? "armada-popover__panel armada-popover__panel--end"
              : "armada-popover__panel armada-popover__panel--start"
          }
          role="dialog"
          aria-label={label}
        >
          {typeof children === "function" ? children(() => setOwn(false)) : children}
        </div>
      ) : null}
    </div>
  );
}
