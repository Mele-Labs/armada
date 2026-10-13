import { Square } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/** Ends the turn that is running, beside the working mark. The Session then rests, and its box takes the next message. */
export function SessionStop({ onStop }: { onStop: () => void }) {
  return (
    <Tooltip label="Stop">
      <Button
        variant="ghost"
        size="sm"
        iconOnly
        aria-label="Stop"
        onClick={(event) => {
          // On a folded tool group's head this press must not open the group.
          event.preventDefault();
          event.stopPropagation();
          onStop();
        }}
      >
        <Square size={16} strokeWidth={2} aria-hidden />
      </Button>
    </Tooltip>
  );
}
