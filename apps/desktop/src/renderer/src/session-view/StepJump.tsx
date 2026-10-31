// Jumping a Job to a step to come: the icon in that step's band, beside Skip. Fleet will move the Job there and skip the steps between.

import { CornerDownRight } from "lucide-react";
import { Button, Tooltip } from "@armada/components";

export const JUMP_SAID = "Jump here";

export function JumpHere({ onPress }: { onPress: () => void }) {
  return (
    <Tooltip label={JUMP_SAID} asChild>
      <Button variant="ghost" size="sm" iconOnly aria-label={JUMP_SAID} onClick={onPress}>
        <CornerDownRight size={16} aria-hidden="true" />
      </Button>
    </Tooltip>
  );
}
