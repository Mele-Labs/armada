import { createContext, useContext, useMemo, useState, type ReactNode } from "react";
import { useNow } from "../../clock";
import { ageOf, stampOf, useTimeFormat } from "../../time-format";
import { Tooltip } from "../Tooltip/Tooltip";

/**
 * A time, drawn in the format set in Settings, and a press that reads it as an age instead ("4m ago").
 * It is a native button drawn as the text around it, so Enter and Space flip it and the global focus ring
 * shows. **The tooltip is the other reading**, which is what a press gives.
 *
 * The age is redrawn each quarter minute, which is as fine as it is read.
 */
export type StampProps = {
  /** An RFC 3339 instant, or epoch milliseconds. Text that is neither is drawn as it came, and is no button. */
  at: string | number;
  /** Which reading is drawn before a press. Places that show ages today keep `age`. */
  first?: "clock" | "age";
  /**
   * Drawn as plain text, for a stamp inside a button or a summary, where a button of its own would be a
   * nested one. It follows its surface's flip and the format like any other, and has no press of its own.
   */
  still?: boolean;
  /** The instant ages are read against, where the host holds its own clock. Absent, the window's. */
  now?: number;
  className?: string;
};

type Surface = { flipped: boolean; flip: () => void };
const SurfaceContext = createContext<Surface | null>(null);

/**
 * One surface's stamps, flipped together: a press on any inside turns every one inside, and a second
 * press turns them back. **A stamp outside a surface turns only itself.** Other screens keep their own.
 */
export function StampSurface({ children }: { children: ReactNode }) {
  const [flipped, setFlipped] = useState(false);
  const value = useMemo<Surface>(() => ({ flipped, flip: () => setFlipped((was) => !was) }), [flipped]);
  return <SurfaceContext.Provider value={value}>{children}</SurfaceContext.Provider>;
}

export function Stamp({ at, first = "clock", still = false, now: given, className }: StampProps) {
  const format = useTimeFormat();
  const ticking = useNow(15_000);
  const now = given ?? ticking;
  const surface = useContext(SurfaceContext);
  const [own, setOwn] = useState(false);
  const flipped = surface?.flipped ?? own;
  const ms = typeof at === "number" ? at : Date.parse(at);
  const classes = className === undefined ? "armada-stamp" : `armada-stamp ${className}`;
  if (Number.isNaN(ms)) return <span className={classes}>{at}</span>;
  const age = ageOf(ms, now);
  const clock = stampOf(ms, format);
  const reading = (first === "age") !== flipped ? age : clock;
  if (still) return <span className={classes}>{reading}</span>;
  return (
    <Tooltip label={reading === age ? clock : age} asChild>
      <button
        type="button"
        className={classes}
        onClick={(event) => {
          // A stamp in a row that opens on a press turns the stamps, and does not open the row.
          event.stopPropagation();
          (surface?.flip ?? (() => setOwn((was) => !was)))();
        }}
      >
        {reading}
      </button>
    </Tooltip>
  );
}
