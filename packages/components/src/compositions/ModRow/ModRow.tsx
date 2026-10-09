import type { ReactNode } from "react";
import { GitPullRequestArrow, Palette } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Switch } from "../../primitives/Switch/Switch";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * One mod on this machine: the kind it is, its switch, and Promote.
 *
 * **A theme is the only kind there is**, so the leading mark is the palette with its name on
 * hover. The line under the title is one plain fact: what is wrong with the mod, or the branch it
 * was put on. Nothing is said where there is nothing to say.
 */
export type ModRowProps = {
  /** The mod's name. Names the row and the switch. */
  title: string;
  /** This machine's switch for the mod. */
  enabled: boolean;
  onEnabled: (on: boolean) => void;
  /** Why the mod cannot be drawn, or the branch it is on. Absent draws no line. */
  detail?: ReactNode;
  /** Promote is off while the mod is invalid or already on a branch. */
  promoteDisabled?: boolean;
  onPromote: () => void;
};

export function ModRow({ title, enabled, onEnabled, detail, promoteDisabled = false, onPromote }: ModRowProps) {
  return (
    <div className="armada-mod-row" role="group" aria-label={title}>
      <Tooltip label="Theme">
        <span className="armada-mod-row__kind" role="img" aria-label="Theme">
          <Palette size={16} />
        </span>
      </Tooltip>
      <div className="armada-mod-row__switch">
        <Switch checked={enabled} onChange={(event) => onEnabled(event.target.checked)} {...(detail === undefined ? {} : { description: detail })}>
          {title}
        </Switch>
      </div>
      <Tooltip label={`Put ${title} on a branch of the repository`}>
        <Button iconOnly variant="ghost" aria-label={`Promote ${title}`} disabled={promoteDisabled} onClick={onPromote}>
          <GitPullRequestArrow size={16} />
        </Button>
      </Tooltip>
    </div>
  );
}
