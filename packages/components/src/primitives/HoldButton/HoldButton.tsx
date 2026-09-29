import { useId, type ReactNode } from "react";
import { Button, type ButtonProps } from "../Button/Button";
import { useHold } from "./useHold";

/**
 * A destructive act that confirms in place: it fills while held and commits
 * once held for `--duration-hold`. The rule is the design system contract's,
 * under Keyboard and command palette, "A hold confirms in place".
 *
 * Built on `Button`'s destructive variant, so outline, focus, disabled and
 * pending are that file's — or on its ghost icon-only face, where `icon` asks.
 */
export type HoldButtonProps = Omit<
  ButtonProps,
  | "variant"
  | "iconOnly"
  | "ref"
  | "children"
  | "onClick"
  | "onPointerDown"
  | "onPointerUp"
  | "onPointerLeave"
  | "onPointerCancel"
  | "onKeyDown"
  | "onKeyUp"
  | "onBlur"
  | "aria-describedby"
> & {
  /** The label while the hold is offered, naming the hold: `Hold to kill job`. */
  children: string;
  /** The label where the hold is not offered and a press asks: `Kill job`. */
  askLabel: string;
  /** What holding does, read to a person who cannot see the fill. */
  description: string;
  /** Held for the whole duration. Called once per hold. */
  onCommit: () => void;
  /** Pressed where the hold is not offered. Opens the confirmation. */
  onAsk: () => void;
  /**
   * Draw this glyph alone, as a ghost icon-only row act. `children` and
   * `askLabel` become its accessible name, since the glyph is the whole face.
   * **Ghost and not destructive**: icon-only is a ghost affordance, and a red
   * outline on every row of a list would be the loudest thing on it. The fill
   * still grows under the glyph, so the hold reads the same.
   */
  icon?: ReactNode;
};

export { holdDurationOf, useHold, usePrefersReducedMotion } from "./useHold";
export type { Hold, HoldHandlers, HoldPhase } from "./useHold";

export function HoldButton({
  children,
  askLabel,
  description,
  onCommit,
  onAsk,
  disabled,
  pending,
  icon,
  ...rest
}: HoldButtonProps) {
  const describedBy = useId();
  const hold = useHold<HTMLButtonElement>({
    inert: disabled === true || pending === true,
    onCommit,
    onAsk,
  });

  const face =
    icon === undefined ? ({ variant: "destructive" } as const) : ({ variant: "ghost", iconOnly: true } as const);

  // Under reduced motion the fill is invisible, and it is the only thing that
  // says how long is left, so the press asks instead.
  if (!hold.offered) {
    return (
      <Button
        {...rest}
        {...face}
        {...(icon === undefined ? {} : { "aria-label": askLabel })}
        disabled={disabled}
        pending={pending}
        onClick={onAsk}
      >
        {icon ?? askLabel}
      </Button>
    );
  }

  // No `onClick`: a click is a press and release, and a press alone never commits.
  return (
    <Button
      {...rest}
      {...hold.handlers}
      ref={hold.ref}
      {...face}
      {...(icon === undefined ? {} : { "aria-label": children })}
      disabled={disabled}
      pending={pending}
      data-hold=""
      aria-describedby={describedBy}
    >
      <span className="armada-hold__fill" data-phase={hold.phase} aria-hidden="true" />
      <span className="armada-hold__label">{icon ?? children}</span>
      {/* Hidden and still read: a description follows its reference into hidden content. */}
      <span id={describedBy} hidden>
        {description}
      </span>
    </Button>
  );
}
