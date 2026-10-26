import { AppWindowMac, Check, CircleDashed, LoaderCircle, Package, Power, X } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { PHASE_SAID } from "./runs";
import type { RunPhase } from "./runs";

const GLYPH: Record<RunPhase, LucideIcon> = { "setting-up": Package, waiting: CircleDashed, starting: LoaderCircle, ready: Check, exited: Power, failed: X };

/**
 * A run's state as one registry glyph with its name in a tooltip. `said` leads the tooltip where the glyph stands apart from a name, as on a repository's row.
 * `names` says the phases in other words, as a setup Command's are.
 */
export function RunPhaseMark({ phase, said, size = 12, names = PHASE_SAID }: { phase: RunPhase; said?: string; size?: 12 | 16; names?: Readonly<Record<RunPhase, string>> }) {
  const Glyph = GLYPH[phase];
  const label = said === undefined ? names[phase] : `${said}, ${names[phase].toLowerCase()}`;
  return (
    <Tooltip label={label}>
      <span className="armada-run-mark" data-phase={phase} role="img" aria-label={label}>
        <Glyph size={size} strokeWidth={2} aria-hidden="true" />
      </span>
    </Tooltip>
  );
}

/** The same mark as a press that opens the run it is the state of: a repository's row and a Job's header, where the run is not on screen. */
export function RunPhaseButton({ phase, said, onPress }: { phase: RunPhase; said: string; onPress: () => void }) {
  const Glyph = GLYPH[phase];
  const label = `${said}, ${PHASE_SAID[phase].toLowerCase()}`;
  return (
    <Tooltip label={label}>
      <Button variant="ghost" size="sm" iconOnly aria-label={label} onClick={onPress}>
        <span className="armada-run-mark" data-phase={phase} aria-hidden="true">
          <Glyph size={16} strokeWidth={2} aria-hidden="true" />
        </span>
      </Button>
    </Tooltip>
  );
}

/** A desktop app's window is open: stands where a browser run has its Open. */
export function RunWindowMark({ phase }: { phase: RunPhase }) {
  const label = phase === "ready" ? "Window open" : phase === "starting" ? "Window opening" : "Window closed";
  return (
    <Tooltip label={label}>
      <span className="armada-run-mark" data-phase={phase === "ready" || phase === "starting" ? "ready" : "exited"} role="img" aria-label={label}>
        <AppWindowMac size={16} strokeWidth={2} aria-hidden="true" />
      </span>
    </Tooltip>
  );
}
