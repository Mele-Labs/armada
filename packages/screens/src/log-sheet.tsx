// The panel a log is read in, while it is written and after. Lifted out of Pulse's log panel
// (`pulse-log-sheet.tsx`) when a Check's log came to need the same one (owner, 2 Oct 2026:
// *click on the specific check to open a panel and see the live log*), so there is one log panel
// and every log reads the same: what it is of, a pulsing mark while it is written, the tail
// followed, wrap where its lines are console lines, and `Esc`.
//
// **It draws the frame and never the lines.** What a log holds is its reader's, handed in as
// `children`: a Job's notes, a Drone's turns, a brief or a Check's console output.

import { useLayoutEffect, useRef, type ReactNode } from "react";

import { ChevronRight } from "lucide-react";

import { BeingWritten, Button, ConsoleWrapToggle, Sheet, Tooltip } from "@armada/components";

export type LogSheetProps = {
  /** Which kind of panel this is, for the width it remembers. `Sheet`'s own `kind`. */
  kind: string;
  title: string;
  /** What the log is of, under the title. */
  about: ReactNode;
  /** Still being written: the mark beside `about`, and the tail followed. */
  live: boolean;
  /** Changes as lines arrive, so the tail is followed while `live`. */
  grows: number;
  /** The wrap toggle, where the lines are console lines. Absent draws none. */
  wrap?: { wrap: boolean; onToggle: () => void };
  /**
   * A way on to where this log's record is read, icon-only and named by its tooltip: a Check's
   * log goes to the Check's own Record row (owner, 2 Oct 2026). Absent draws none, never a
   * control that is off.
   */
  goes?: { label: string; onGo: () => void };
  /** Bands under the head that stay while the log scrolls: the facts of what it is of. */
  bands?: ReactNode;
  /**
   * Inside the screen it was opened from, or over the whole work area. Pulse's panel sits inside
   * its tab; a Check's opens from a card or a row anywhere, so it floats, as Record's and Drones'
   * sheets do.
   */
  placement: "contained" | "floating";
  floor: boolean;
  onClose: () => void;
  children: ReactNode;
};

export function LogSheet({ kind, title, about, live, grows, wrap, goes, bands, placement, floor, onClose, children }: LogSheetProps) {
  const body = useRef<HTMLDivElement>(null);
  // The tail, while it is written. A reader that follows its own (`DroneTurns`) is not moved by
  // this either way: it is already at the bottom.
  useLayoutEffect(() => {
    const el = body.current;
    if (live && el !== null) el.scrollTop = el.scrollHeight;
  }, [live, grows]);
  return (
    <Sheet
      kind={kind}
      open
      {...(placement === "floating" ? { floating: true } : { contained: true })}
      size="wide"
      floor={floor}
      title={title}
      subtitle={
        <>
          {about}
          {live ? (
            <>
              {" "}
              <BeingWritten />
            </>
          ) : null}
        </>
      }
      {...(wrap === undefined && goes === undefined
        ? {}
        : {
            controls: (
              <>
                {wrap === undefined ? null : <ConsoleWrapToggle wrap={wrap.wrap} onToggle={wrap.onToggle} />}
                {goes === undefined ? null : (
                  // `chevron-right`'s goes-to, alone: the tooltip is its label.
                  <Tooltip label={goes.label} asChild>
                    <Button variant="ghost" size="sm" iconOnly aria-label={goes.label} onClick={goes.onGo}>
                      <ChevronRight size={16} strokeWidth={2} aria-hidden="true" />
                    </Button>
                  </Tooltip>
                )}
              </>
            ),
          })}
      {...(bands === undefined ? {} : { bands })}
      closeLabel="Close"
      closeBinding="Esc"
      bodyRef={body}
      onClose={onClose}
    >
      {children}
    </Sheet>
  );
}
