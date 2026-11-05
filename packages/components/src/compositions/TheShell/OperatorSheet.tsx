import { Bot } from "lucide-react";
import type { ReactNode } from "react";
import { Sheet } from "../../primitives/Sheet/Sheet";

/**
 * The Operator's dock, folded into a sheet below the layout breakpoint. **It keeps
 * the dock's frame** (#1320): opened on a narrow window, it is still the same
 * place it is beside the content, not one more sheet.
 *
 * The chip is the dock's own — same class, same glyph, in `Sheet`'s `leading`
 * slot. `--operator` on `--operator-muted` over the sheet's `--bg-overlay` reads
 * 4.65:1, over the 3:1 a non-text mark takes.
 *
 * The wrapper is `display: contents`, so the scrim still lays out against
 * `.armada-shell__work`. It exists only to scope the frame in `TheShell.css`
 * to this sheet; every other sheet keeps the primitive's look.
 */
export function OperatorSheet({
  open,
  title,
  binding,
  width,
  onResize,
  onClose,
  children,
}: {
  open: boolean;
  title: string;
  binding?: string;
  /** The dock's own width and resize, so a drag here sets the dock's too (owner, 2 Oct 2026). */
  width?: number | undefined;
  onResize?: ((width: number) => void) | undefined;
  onClose: () => void;
  children: ReactNode;
}) {
  return (
    <div className="armada-shell__operator-sheet">
      <Sheet
        {...(onResize === undefined
          ? {
              kind: "operator",
            }
          : {
              width,
              onResize,
            })}
        open={open}
        title={title}
        leading={
          <span className="armada-shell__dock-chip" aria-hidden>
            <Bot size={16} strokeWidth={2} />
          </span>
        }
        contained
        closeLabel="Close"
        closeBinding={binding}
        onClose={onClose}
      >
        {children}
      </Sheet>
    </div>
  );
}
