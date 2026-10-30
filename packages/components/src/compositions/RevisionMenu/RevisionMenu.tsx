import { DropdownMenu, type DropdownMenuEntry } from "../../primitives/DropdownMenu/DropdownMenu";

export type RevisionMenuProps = {
  /** How many revisions there are. The newest is `count`. */
  count: number;
  /** The one shown, from 1 to `count`. */
  revision: number;
  onPick: (revision: number) => void;
  /** Held by the caller, for a sheet that must know a menu is open so its Esc closes the menu first. */
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  /** Which of the trigger's edges the menu lines up with: `start` in a sheet's subtitle, `end` at a card's right edge. */
  align?: "start" | "end";
  /** As `DropdownMenu`'s: an element for a trigger inside a floating sheet, whose scrim would lie over a panel drawn on the body. */
  portal?: boolean | HTMLElement;
};

/**
 * Which revision of a thing a Job kept is shown, newest first: a document's pages, a sketch's redraws. **The trigger names the
 * revision shown**, so the newest reads as a number too, and its tooltip says what the number is.
 */
export function RevisionMenu({ count, revision, onPick, open, onOpenChange, align = "start", portal = false }: RevisionMenuProps) {
  const entries = Array.from({ length: count }, (_, at): DropdownMenuEntry => ({ kind: "item", id: String(at + 1), label: `Revision ${String(at + 1)}`, selected: at + 1 === revision })).reverse();
  return (
    <span className="armada-revision-menu">
      <DropdownMenu
        plain
        align={align}
        portal={portal}
        triggerLabel={`Revision ${String(revision)}`}
        triggerTooltip="Revision"
        entries={entries}
        {...(open === undefined ? {} : { open })}
        {...(onOpenChange === undefined ? {} : { onOpenChange })}
        onSelect={(id) => onPick(Number(id))}
      />
    </span>
  );
}
