import { Fragment, useId, useState } from "react";
import type { ReactNode } from "react";
import { ChevronDown } from "lucide-react";
import type { ButtonAnswer } from "../Button/Button";
import { Tooltip } from "../Tooltip/Tooltip";
import { useHold } from "../HoldButton/useHold";

/**
 * The likely action, with the rest one click away — a button and a dropdown
 * menu in one control, not a new primitive.
 *
 * The label is always the action a person is most likely to take from that
 * state, so it changes with the Job, and it never repeats inside the menu. A
 * split button with nothing in its menu is a button.
 *
 * The caret is not a label icon. It is the whole content of its own divided
 * segment, structural rather than decorative — the one exception to
 * label-only buttons, and the only thing `chevron-down` is granted here.
 *
 * A row carries this or an ellipsis, never both: the ellipsis means there is
 * no likely action, this means there is and it is the label.
 */
export type SplitButtonItem = {
  /** The verb, sentence case. Never a repeat of the label, and never "Open". */
  label: string;
  /** The single-key or modifier binding, if the action has one. */
  shortcut?: string;
  /** Destructive. Last in the list, `--status-completed-failed` text. */
  danger?: boolean;
  /**
   * What choosing this does, on hover over the entry. Absent draws none, which
   * is every entry whose verb is the whole of it.
   *
   * **An entry behind a caret is the one place a label is read without the act
   * beside it.** The review gate's Approve and Reject sit here — the owner's
   * own arrangement, 30 Sep 2026 — and what each leaves behind is the
   * difference a person has to know before choosing, so the sentence that used
   * to sit on a button of its own comes with the act rather than being dropped.
   */
  note?: ReactNode;
  onSelect?: () => void;
};

export type SplitButtonProps = {
  /** The act the state calls for. */
  children: string;
  /**
   * A leading glyph on the label segment, from `packages/icons/icons.toml`
   * only. Absent draws none — most callers have no icon to lead with, since a
   * list row's label already carries the act. The title row's Dispatch is the
   * one caller today (#1107).
   */
  icon?: ReactNode;
  /** What the row could also do. Destructive last. */
  items: SplitButtonItem[];
  /**
   * Secondary on a list row, always. `primary` is legal where a surface has one
   * primary: Job detail's header, and the Board's head.
   *
   * `destructive` is outlined, never filled — a solid red control reads as an
   * error state rather than as an act. It is for a group whose every member
   * ends something, so that the caret cannot make a terminal act look like a
   * variant of the one on the face.
   *
   * `tonal` is chrome, not a list row's act: `--accent-muted` fill,
   * `--accent-hover` text, no outer border — only the inner divider between
   * segments. The title row's Dispatch is the one caller (#1087's correction
   * pass), always paired with `items={[]}` below.
   */
  variant?: "secondary" | "primary" | "destructive" | "tonal";
  /** The surface underneath: `card` fills `--bg-sunken`, `sunken` `--bg-raised`. Read by `secondary` only. */
  ground?: "card" | "sunken";
  /**
   * `default` is `--h-control` (36px), every list row and Job detail's own
   * header. `sm` is `--h-control-sm` (32px), for a control sitting beside a
   * field that height already binds — the title row's search field is the one
   * caller.
   */
  size?: "default" | "sm";
  /** Render with the menu open. Uncontrolled otherwise. */
  defaultOpen?: boolean;
  disabled?: boolean;
  /**
   * The face's own act is unavailable while everything behind the caret still
   * is. **Not `disabled`, which takes the whole control**, and the difference
   * is the point: a merge Fleet would refuse must not be pressable, and
   * Approve, which the same caret discloses, must still be. The control paints
   * as a disabled one — a greyed face has no variant left to express — and the
   * caret alone stays live, which is what says there is another act here.
   */
  faceDisabled?: boolean;
  /**
   * The id of the sentence saying why the face is off, named on the face
   * rather than read from beside it. **The reason is the caller's to draw and
   * to place**: it is a whole sentence, and inside the control it would widen
   * the face's column. `ReviewDecision` puts it under the row.
   */
  faceDescribedBy?: string;
  /**
   * What pressing the face does, on hover over it. Absent draws none.
   *
   * **The face only, never the caret.** A tooltip names the control it is on
   * and never the one beside it — the design system contract, under Tooltip —
   * so the caret keeps `menuLabel` and each entry carries its own `note`.
   *
   * **Not for a face that holds.** `useHold` takes `onBlur` to cancel a hold
   * caught by focus leaving, and a tooltip takes the same handler to close; no
   * caller pairs them and this does not try to reconcile the two.
   */
  note?: ReactNode;
  /**
   * The act this control opened a dialog for is out, and Fleet has not
   * answered — the same reading as `Button`'s own `pending`, on the one
   * visible surface a split button still has once its menu has closed. The
   * face sweeps a bar and stays focusable; the caret goes off and the menu
   * will not open, since there is nothing else to disclose while a press is
   * already on its way. #1117.
   */
  pending?: boolean;
  /** Fleet's answer to the face's press, drawn as `Button`'s own `answer`. Pending wins. */
  answer?: ButtonAnswer;
  /** The face's label while `pending` holds. Falls back to `children`. */
  pendingLabel?: string;
  /**
   * Called on a press of the face. **Where `hold` is set, only where the hold is
   * not offered** — under reduced motion, or where `--duration-hold` cannot be
   * read — so it is the act that asks rather than the one that sends.
   */
  onAction?: () => void;
  /**
   * The face confirms in place: it fills while held and calls `onCommit` once
   * held for `--duration-hold`, as `HoldButton` does and through the same
   * `useHold`. The design system contract, "A hold confirms in place".
   *
   * **The face only.** The caret stays a menu trigger and never starts a hold,
   * and every entry behind it keeps its own `onSelect`. `children` is the label
   * where the hold is not offered and a press calls `onAction` instead.
   */
  hold?: {
    /** The face's label while the hold is offered, naming the hold: `Hold to kill drone`. */
    label: string;
    /** What holding does, read to a person who cannot see the fill. */
    description: string;
    onCommit: () => void;
  };
  /** Names the menu for a reader who cannot see the caret. Also the caret's own name where `items` is empty. */
  menuLabel?: string;
};

export function SplitButton({
  children,
  icon,
  items,
  variant = "secondary",
  ground = "card",
  size = "default",
  defaultOpen = false,
  disabled = false,
  faceDisabled = false,
  faceDescribedBy,
  note,
  pending = false,
  answer,
  pendingLabel,
  onAction,
  hold,
  menuLabel = "More actions",
}: SplitButtonProps) {
  const [open, setOpen] = useState(defaultOpen);
  const describedBy = useId();
  // Called on every render so the hook order never changes; inert where there is
  // no hold to offer.
  const held = useHold<HTMLButtonElement>({
    inert: hold === undefined || disabled || pending,
    onCommit: () => hold?.onCommit(),
    onAsk: () => onAction?.(),
  });
  // Pending wins: the act is already out, so the face is its bar and nothing to hold.
  const holding = hold !== undefined && held.offered && !pending;
  // Nothing behind the caret yet — the title row's Dispatch has no menu, and
  // both segments call the same handler. A menu with nothing in it is a floating
  // empty box, not an absence, so the caret has to stop opening one rather than
  // opening one with nothing to show.
  const noMenu = items.length === 0;

  const face = (
    <button
      type="button"
      className="armada-split-button__action"
      data-pending={pending || undefined}
      data-answer={pending ? undefined : answer}
      // Not `disabled`, on `Button`'s own reasoning: a disabled control
      // drops focus and is skipped by a screen reader, and this is the one
      // still standing for the press that is out.
      disabled={pending ? undefined : disabled || faceDisabled}
      aria-disabled={pending || undefined}
      aria-busy={pending || undefined}
      {...(faceDescribedBy === undefined || holding ? {} : { "aria-describedby": faceDescribedBy })}
      {...(holding
        ? {
            ...held.handlers,
            ref: held.ref,
            "data-hold": "",
            // Both, where the caller named one: a hold's own description and
            // the reason the act is off are two facts about the same control.
            "aria-describedby":
              faceDescribedBy === undefined ? describedBy : `${faceDescribedBy} ${describedBy}`,
          }
        : { onClick: pending ? undefined : onAction })}
    >
      {holding ? (
        <>
          <span className="armada-hold__fill" data-phase={held.phase} aria-hidden="true" />
          {icon}
          <span className="armada-hold__label">{hold.label}</span>
          {/* Hidden and still read: a description follows its reference into hidden content. */}
          <span id={describedBy} hidden>
            {hold.description}
          </span>
        </>
      ) : (
        <>
          {icon}
          {pending ? (pendingLabel ?? children) : children}
        </>
      )}
    </button>
  );

  return (
    <div className="armada-split-button">
      <div
        className="armada-split-button__control"
        data-variant={variant}
        data-ground={ground}
        data-size={size === "default" ? undefined : size}
      >
        {/* `asChild`, because the face is a flex item of the control: a `span`
            around it would take the segment out of the group it is half of.
            The bubble is `position: fixed` against the face's own anchor, so
            the face's `overflow: clip` — there for the press line's corners —
            does not reach it. */}
        {note === undefined ? face : <Tooltip asChild label={note}>{face}</Tooltip>}
        <button
          type="button"
          className="armada-split-button__caret"
          aria-haspopup={noMenu ? undefined : "menu"}
          aria-expanded={noMenu ? undefined : pending ? false : open}
          aria-label={menuLabel}
          // `faceDisabled` reaches the caret only where the caret *is* the
          // face: with no menu behind it both segments send the one act, so
          // leaving it live would be the blocked press by another door.
          disabled={disabled || pending || (noMenu && faceDisabled)}
          onClick={() => (noMenu ? onAction?.() : setOpen((was) => !was))}
        >
          <ChevronDown size={16} strokeWidth={2} aria-hidden="true" />
        </button>
      </div>
      {!noMenu && open && !pending && (
        <div className="armada-split-button__menu" role="menu" aria-label={menuLabel}>
          {items.map((item) => {
            const entry = (
              <button
                type="button"
                role="menuitem"
                className="armada-split-button__item"
                data-danger={item.danger || undefined}
                // Closes on choosing. A menu still open over the confirmation it
                // just raised is a control that did not respond.
                onClick={() => {
                  setOpen(false);
                  item.onSelect?.();
                }}
              >
                <span>{item.label}</span>
                {item.shortcut !== undefined && (
                  <span className="armada-split-button__shortcut">{item.shortcut}</span>
                )}
              </button>
            );
            return item.note === undefined ? (
              <Fragment key={item.label}>{entry}</Fragment>
            ) : (
              <Tooltip key={item.label} asChild label={item.note}>
                {entry}
              </Tooltip>
            );
          })}
        </div>
      )}
    </div>
  );
}
