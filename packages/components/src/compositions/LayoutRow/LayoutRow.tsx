import type { LucideIcon } from "lucide-react";
import { ArrowDown, ArrowUp, Puzzle } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Switch } from "../../primitives/Switch/Switch";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * One tab, panel or rail row a layout arranges: its glyph, a switch for whether it is drawn, and
 * the two moves. **A row that cannot be hidden has no switch**, only its name with `Always shown` on
 * hover, and a row a mod hid carries the puzzle with the mod's name on hover. Nothing else is said.
 */
export type LayoutRowProps = {
  label: string;
  icon: LucideIcon;
  visible: boolean;
  /** False for anything the owner has to decide on, or has to reach to undo a layout. */
  hideable: boolean;
  onVisible: (on: boolean) => void;
  /** The mod that hid it, where one did. */
  hiddenBy?: string;
  /** Absent where it cannot move: first, or a list a layout does not order. */
  onUp?: () => void;
  onDown?: () => void;
};

export function LayoutRow({ label, icon: Icon, visible, hideable, onVisible, hiddenBy, onUp, onDown }: LayoutRowProps) {
  return (
    <div className="armada-layout-row" role="group" aria-label={label}>
      <span className="armada-layout-row__mark">
        <Icon size={16} aria-hidden="true" />
      </span>
      <div className="armada-layout-row__switch">
        {hideable ? (
          <Switch checked={visible} onChange={(event) => onVisible(event.target.checked)}>
            {label}
          </Switch>
        ) : (
          <Tooltip label="Always shown">
            <span className="armada-layout-row__fixed">{label}</span>
          </Tooltip>
        )}
      </div>
      {hiddenBy === undefined ? null : (
        <Tooltip label={hiddenBy}>
          <span className="armada-layout-row__from" role="img" aria-label={`Hidden by ${hiddenBy}`}>
            <Puzzle size={16} />
          </span>
        </Tooltip>
      )}
      {onUp === undefined && onDown === undefined ? null : (
        <>
          <Tooltip label="Move up">
            <Button iconOnly variant="ghost" aria-label={`Move ${label} up`} disabled={onUp === undefined} onClick={onUp}>
              <ArrowUp size={16} />
            </Button>
          </Tooltip>
          <Tooltip label="Move down">
            <Button iconOnly variant="ghost" aria-label={`Move ${label} down`} disabled={onDown === undefined} onClick={onDown}>
              <ArrowDown size={16} />
            </Button>
          </Tooltip>
        </>
      )}
    </div>
  );
}
