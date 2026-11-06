// The window while a swap onto the snapshot is under way, and the line it leaves when the swap was undone.
//
// **Fleet stops during a swap**, so the window would draw a lost connection for the minute it takes. From the
// press until the report no longer says `swapping`, and Fleet answers again, the window is dimmed and takes no
// presses, with one banner and the seconds. **Nothing is unmounted**: the dim is a layer over the surface, so
// text typed in a box is where it was left when the layer goes. The report is marked `swapping` on the press
// (`main/fleet-build.ts`) and stays so while Fleet is down, so this needs no state of its own about the swap.
//
// A Fleet that swapped back leaves `failed` as a line starting `Swapped back`. The banner says it until it is put
// away or the next report carries none.

import { useEffect, useState } from "react";
import { Alert, Button, useNowWhile } from "@armada/components";
import type { FleetBuildReport } from "@armada/protocol";
import { LoaderCircle } from "lucide-react";
import "./swap-hold.css";

const SWAPPED_BACK = "Swapped back";

/** What the report says about a swap: whether the window is held, for how long, and the line a swap back left. */
export function useSwapHold(report: FleetBuildReport | null): {
  holding: boolean;
  seconds: number;
  swappedBack: string | null;
  dismiss: () => void;
} {
  const holding = report?.swapping === true;
  const [since, setSince] = useState<number | null>(null);
  const now = useNowWhile(holding);
  useEffect(
    () => setSince(holding ? Date.now() : null),
    [
      holding,
    ],
  );
  const line = report?.failed?.startsWith(SWAPPED_BACK) === true ? report.failed : null;
  const [dismissed, setDismissed] = useState<string | null>(null);
  // A report with no such line forgets the one put away, so a later swap back says its line again.
  useEffect(() => {
    if (line === null) setDismissed(null);
  }, [
    line,
  ]);
  return {
    holding,
    seconds: holding && since !== null ? Math.max(0, Math.floor((now - since) / 1000)) : 0,
    swappedBack: holding || line === null || line === dismissed ? null : line,
    dismiss: () => setDismissed(line),
  };
}

/** The dim and the banner. Draws nothing when the window is neither held nor carrying a swap back. */
export function SwapHold({ report }: { report: FleetBuildReport | null }) {
  const { holding, seconds, swappedBack, dismiss } = useSwapHold(report);
  // A held window takes no keys either: the layer catches clicks, and this catches the keyboard.
  useEffect(() => {
    if (!holding) return;
    const stop = (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey) return;
      event.preventDefault();
      event.stopPropagation();
    };
    window.addEventListener("keydown", stop, true);
    return () => window.removeEventListener("keydown", stop, true);
  }, [
    holding,
  ]);
  if (!holding && swappedBack === null) return null;
  return (
    <>
      {holding ? <div className="armada-swap-hold__dim" data-testid="swap-hold" /> : null}
      <div className="armada-swap-hold__banner">
        {holding ? (
          <Alert
            tone="neutral"
            title="Swapping to the build"
            icon={
              <LoaderCircle
                className="armada-swap-hold__spin"
                size={16}
                strokeWidth={2}
                aria-hidden="true"
              />
            }
          >
            {`${seconds} s`}
          </Alert>
        ) : (
          <Alert
            tone="caution"
            title="Swapped back"
            action={
              <Button variant="ghost" size="sm" onClick={dismiss}>
                Dismiss
              </Button>
            }
          >
            {swappedBack?.slice(SWAPPED_BACK.length).replace(/^[\s:,.-]+/, "")}
          </Alert>
        )}
      </div>
    </>
  );
}
