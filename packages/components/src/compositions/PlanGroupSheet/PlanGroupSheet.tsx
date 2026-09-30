import { Button } from "../../primitives/Button/Button";
import { CardContent } from "../../primitives/Card/Card";
import { Sheet } from "../../primitives/Sheet/Sheet";
import {
  PlanGroupBody,
  type PlanBoardAdd,
  type PlanBoardGroup,
  type PlanBoardProps,
} from "../PlanBoard/PlanBoard";

/**
 * One plan group, pressed on the Plan graph — its task rows, its Checks and
 * Tests and its overlap warning, with Add task in the head (owner, 30 Sep
 * 2026: "So I can only add a task in list view?").
 *
 * **The list's card, on the task panel's layer.** The body is `PlanGroupBody`,
 * the same drawing the list's card holds, laid out by the same `CardContent`;
 * the frame is the floating `Sheet` `PlanTaskSheet` opens, dimming the work
 * area and resizing by the same handle.
 *
 * **Everything the list's card can do, this can** (owner, 30 Sep 2026: "graph
 * and list should offer the same features"): Add task, the group's asks, and
 * Propose a change, each on the list's own terms.
 */
export type PlanGroupSheetProps = {
  open: boolean;
  group: PlanBoardGroup;
  /** Pressing a task row. The caller opens its panel in place of this one. */
  onOpenTask?: (taskId: string) => void;
  /** Absent draws no Add task. */
  add?: PlanBoardAdd;
  /** The group's asks, as the list's card offers them. `group.asks` empty draws none. */
  onAsk?: (groupId: string, askId: string) => void;
  askPending?: boolean;
  /** Proposing a change to this group, beside its asks. */
  propose?: PlanBoardProps["propose"];
  floor?: boolean;
  /** `PlanTaskSheet`'s pair: absent draws `--w-dock`, no `onResize` no handle. */
  width?: number;
  onResize?: (width: number) => void;
  /** Another layer — the add dialog — lies over this one and takes `Esc` first. */
  under?: boolean;
  onClose?: () => void;
};

export function PlanGroupSheet({
  open,
  group,
  onOpenTask,
  add,
  onAsk,
  askPending = false,
  propose,
  floor = false,
  width,
  onResize,
  under = false,
  onClose,
}: PlanGroupSheetProps) {
  return (
    <Sheet
      open={open}
      floating
      {...(onResize === undefined ? {} : { width, onResize })}
      floor={floor}
      title={`Group ${group.ordinal}`}
      subtitle={
        <span className="armada-plan-board__state" data-state={group.state}>
          {group.says}
        </span>
      }
      controls={
        add === undefined ? undefined : (
          <Button variant="ghost" size="sm" disabled={add.disabled} onClick={() => add.onAdd(group.id)}>
            {add.label}
          </Button>
        )
      }
      closeLabel="Close"
      closeBinding="Esc"
      under={under}
      onClose={onClose}
    >
      <CardContent className="armada-plan-group-sheet__body">
        <PlanGroupBody
          group={group}
          askPending={askPending}
          guide
          {...(onOpenTask === undefined ? {} : { onOpenTask })}
          {...(onAsk === undefined ? {} : { onAsk })}
          {...(propose === undefined ? {} : { propose })}
        />
      </CardContent>
    </Sheet>
  );
}
