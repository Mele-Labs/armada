// Copy debug report: Doctor's control and the palette's row both open one sheet, which reads the report and
// shows exactly the text Copy puts on the clipboard. Nothing is sent anywhere. **Mock only for now**: a window with
// no source draws neither the control nor the row, so the real app is unchanged until Fleet serves the text.

import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import { CopyAct, Sheet, SkeletonText } from "@armada/components";

import "./debug-report.css";

/** One reading of the report: its text, and how long Fleet takes to answer. `Infinity` never answers. */
export type DebugReading = { text: string; after?: number };

/** What a window offers: open the sheet, and give back focus to `from` when it closes. */
type Offer = { open: (from?: HTMLElement | null) => void };

const Offered = createContext<Offer | null>(null);

/** The offer, or `null` where this window has no report to read. */
export const useDebugReport = (): Offer | null => useContext(Offered);

/** Each opening takes the next reading, and the last one repeats. Absent offers nothing. */
export function DebugReportFrom({ readings, children }: { readings: readonly DebugReading[] | undefined; children: ReactNode }) {
  return readings === undefined || readings.length === 0 ? <>{children}</> : <Held readings={readings}>{children}</Held>;
}

// The widths of a report's lines, so the placeholder has the shape of what lands.
const SHAPE = ["32%", "54%", "62%", "48%", "58%", "40%", "64%", "36%", "52%", "60%", "44%", "56%"];

function Held({ readings, children }: { readings: readonly DebugReading[]; children: ReactNode }) {
  const [open, setOpen] = useState(false);
  const [text, setText] = useState<string>();
  const opened = useRef(0);
  const from = useRef<HTMLElement | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  useEffect(() => () => clearTimeout(timer.current), []);

  const show = useCallback(
    (trigger?: HTMLElement | null) => {
      const reading = readings[Math.min(opened.current, readings.length - 1)]!;
      opened.current += 1;
      from.current = trigger ?? null;
      setText(undefined);
      setOpen(true);
      clearTimeout(timer.current);
      if (reading.after !== Infinity) timer.current = setTimeout(() => setText(reading.text), reading.after ?? 0);
    },
    [readings],
  );

  const close = useCallback(() => {
    clearTimeout(timer.current);
    setOpen(false);
    if (from.current?.isConnected === true) from.current.focus();
  }, []);

  return (
    <Offered.Provider value={{ open: show }}>
      {children}
      <Sheet
        kind="debug-report"
        open={open}
        size="wide"
        title="Debug report"
        closeLabel="Close"
        closeBinding="Esc"
        onClose={close}
        controls={text === undefined ? undefined : <CopyAct text={text} />}
      >
        <div className="armada-debug-report">
          {text === undefined ? <SkeletonText label="Reading the report" widths={SHAPE} /> : <pre className="armada-debug-report__text">{text}</pre>}
        </div>
      </Sheet>
    </Offered.Provider>
  );
}
