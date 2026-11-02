// Copy debug report: Doctor's control and the palette's row both open one sheet, which reads the report and
// shows exactly the text Copy puts on the clipboard. Nothing is sent anywhere. A window with no reader draws neither
// the control nor the row.

import { createContext, useCallback, useContext, useRef, useState, type ReactNode } from "react";
import { CopyAct, Sheet, SkeletonText } from "@armada/components";

import "./debug-report.css";

/** Where the text comes from: one read for each opening. A rejection says why the report could not be read. */
export type DebugRead = () => Promise<string>;

/** What a window offers: open the sheet, and give back focus to `from` when it closes. */
type Offer = {
  open: (from?: HTMLElement | null) => void;
};

const Offered = createContext<Offer | null>(null);

/** The offer, or `null` where this window has no report to read. */
export const useDebugReport = (): Offer | null => useContext(Offered);

/** Each opening reads once. Absent offers nothing. */
export function DebugReportFrom({
  read,
  children,
}: {
  read: DebugRead | undefined;
  children: ReactNode;
}) {
  return read === undefined ? <>{children}</> : <Held read={read}>{children}</Held>;
}

// The widths of a report's lines, so the placeholder has the shape of what lands.
const SHAPE = [
  "32%",
  "54%",
  "62%",
  "48%",
  "58%",
  "40%",
  "64%",
  "36%",
  "52%",
  "60%",
  "44%",
  "56%",
];

/** The sheet's reading: waiting, the text, or why there is none. */
type Reading =
  | {
      state: "reading";
    }
  | {
      state: "read";
      text: string;
    }
  | {
      state: "failed";
      why: string;
    };

function Held({ read, children }: { read: DebugRead; children: ReactNode }) {
  const [open, setOpen] = useState(false);
  const [reading, setReading] = useState<Reading>({
    state: "reading",
  });
  const from = useRef<HTMLElement | null>(null);
  // An answer that arrives after the sheet closed, or after another opening began, is not the one on screen.
  const opening = useRef(0);

  const show = useCallback(
    (trigger?: HTMLElement | null) => {
      const mine = ++opening.current;
      from.current = trigger ?? null;
      setReading({
        state: "reading",
      });
      setOpen(true);
      const settle = (next: Reading) => {
        if (opening.current === mine) setReading(next);
      };
      read().then(
        (text) =>
          settle({
            state: "read",
            text,
          }),
        (error: unknown) =>
          settle({
            state: "failed",
            why: error instanceof Error ? error.message : String(error),
          }),
      );
    },
    [
      read,
    ],
  );

  const close = useCallback(() => {
    opening.current += 1;
    setOpen(false);
    if (from.current?.isConnected === true) from.current.focus();
  }, []);

  return (
    <Offered.Provider
      value={{
        open: show,
      }}
    >
      {children}
      <Sheet
        kind="debug-report"
        open={open}
        size="wide"
        title="Debug report"
        closeLabel="Close"
        closeBinding="Esc"
        onClose={close}
        controls={reading.state === "read" ? <CopyAct text={reading.text} /> : undefined}
      >
        <div className="armada-debug-report">
          {reading.state === "reading" ? (
            <SkeletonText label="Reading the report" widths={SHAPE} />
          ) : reading.state === "read" ? (
            <pre className="armada-debug-report__text">{reading.text}</pre>
          ) : (
            <p className="armada-debug-report__why" role="alert">
              The report could not be read: {reading.why}
            </p>
          )}
        </div>
      </Sheet>
    </Offered.Provider>
  );
}
