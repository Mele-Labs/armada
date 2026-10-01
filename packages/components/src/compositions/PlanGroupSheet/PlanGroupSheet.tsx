import { Button } from "../../primitives/Button/Button";
import { CardContent } from "../../primitives/Card/Card";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { keyFor } from "../../actions";
import {
  PlanGroupBody,
  PlanGroupShape,
  PlanGroupStateWord,
  usePlanMover,
  type PlanBoardAdd,
  type PlanBoardGroup,
  type PlanBoardMove,
  type PlanBoardProps,
  type PlanBoardRemove,
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
 * and list should offer the same features"): Add task, Remove, Propose a
 * change, and dragging a task's row, each on the list's own terms.
 */
export type PlanGroupSheetProps = {
  open: boolean;
  group: PlanBoardGroup;
  /** Every group, in order, so `⌥↑` / `⌥↓` on a task can cross into the next one. */
  groups?: readonly PlanBoardGroup[];
  /** Pressing a task row. The caller opens its panel in place of this one. */
  onOpenTask?: (taskId: string) => void;
  /** Absent draws no Add task. */
  add?: PlanBoardAdd;
  /** Remove, as the list's card offers it. Absent draws none. */
  remove?: PlanBoardRemove;
  /** Moving a task, as the list does. Absent, nothing moves. */
  move?: PlanBoardMove;
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
  groups,
  onOpenTask,
  add,
  remove,
  move,
  askPending = false,
  propose,
  floor = false,
  width,
  onResize,
  under = false,
  onClose,
}: PlanGroupSheetProps) {
  const mover = usePlanMover(groups ?? [group], move);
  const keys = mover.groupKeys(group);
  return (
    <Sheet
      open={open}
      floating
      {...(onResize === undefined ? {} : { width, onResize })}
      floor={floor}
      title={`Group ${group.ordinal}`}
      subtitle={
        <span className="armada-plan-group-sheet__subtitle">
          {/* **The drawing is the group's handle here**: the panel holds one
              group, so there is nowhere to drag it, and `⌥↑` / `⌥↓` on this
              is how Graph moves a group, as the list's head does. */}
          {keys.tabIndex === undefined ? (
            <PlanGroupShape group={group} />
          ) : (
            <Tooltip label="Move group" shortcut={keyFor("move_in_plan")}>
              <span className="armada-plan-group-sheet__handle" aria-label={`Group ${group.ordinal}`} {...keys}>
                <PlanGroupShape group={group} />
              </span>
            </Tooltip>
          )}
          <PlanGroupStateWord state={group.state} says={group.says} />
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
          mover={mover}
          {...(onOpenTask === undefined ? {} : { onOpenTask })}
          {...(remove === undefined ? {} : { remove })}
          {...(propose === undefined ? {} : { propose })}
        />
      </CardContent>
    </Sheet>
  );
}
