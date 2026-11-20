// On a Job's row: this is the person's one priority. Drawn over the prioritised column, so rows line up whether or not they carry it.

import { ArrowUpToLine } from "lucide-react";
import { Tooltip } from "@armada/components";

const SAID = "Prioritised: its builds run first when the machine is loaded";

export function PrioritisedMark() {
  return (
    <Tooltip label={SAID}>
      <span role="img" aria-label="Prioritised">
        <ArrowUpToLine size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}
