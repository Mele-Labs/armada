import type { ComponentProps, HTMLAttributes } from "react";

/**
 * Card — the card treatment under Depth where it sits on the canvas, and flat
 * `--bg-raised` where it does not. Neither is chosen here: the recipe in
 * `src/glass.css` reads the card's ancestry, so a card inside a sheet, a
 * dialog, another card or Helm's dock is flat without its author knowing the
 * rule. `Card.css` says which is which and why.
 *
 * The card owns its padding, so the parts below add none. That is what keeps
 * a header, a body and a footer sharing one left edge.
 */
export type CardProps = ComponentProps<"div">;

/**
 * `flat` is the one ground the recipe's ancestry cannot see: a well, which has
 * no class of its own to be named by. It is an override and not a style — a
 * card inside a sheet, a dialog or another card is already flat.
 */
export type CardSurfaceProps = CardProps & { flat?: boolean };

function joined(base: string, extra?: string) {
  return extra ? `${base} ${extra}` : base;
}

export function Card({ className, flat, ...rest }: CardSurfaceProps) {
  return <div className={joined("armada-card", className)} data-flat={flat || undefined} {...rest} />;
}

/**
 * The legend strip, the header every panel in the app takes: 24px, the title in tracked caps at the left, then an
 * optional `CardReadout` and the panel's 24px ghost icon buttons at the right, and one hairline under it. The Cockpit's
 * `deck-band`, the Logbook's band and the Job's band draw the same strip from the same rule (`depth.css`), so a panel
 * is told by its header and nothing else. State goes on a lamp or a registry glyph, never on a filled header.
 */
export function CardHeader({ className, ...rest }: CardProps) {
  return <div className={joined("armada-card-header", className)} {...rest} />;
}

/** The strip's label. The look sets it in caps, so it is written as a name. */
export function CardTitle({ className, ...rest }: HTMLAttributes<HTMLHeadingElement>) {
  return <h3 className={joined("armada-card-title", className)} {...rest} />;
}

/** The strip's one figure, in mono, before its buttons: a count's place is a tooltip, so this is a reading, never a tally. */
export function CardReadout({ className, ...rest }: HTMLAttributes<HTMLSpanElement>) {
  return <span className={joined("armada-card-readout", className)} {...rest} />;
}

/** Secondary prose beneath the title, at body default in `--fg-muted`. */
export function CardDescription({ className, ...rest }: HTMLAttributes<HTMLParagraphElement>) {
  return <p className={joined("armada-card-description", className)} {...rest} />;
}

export function CardContent({ className, ...rest }: CardProps) {
  return <div className={joined("armada-card-content", className)} {...rest} />;
}

/** Actions. Every button in the group is the same height. */
export function CardFooter({ className, ...rest }: CardProps) {
  return <div className={joined("armada-card-footer", className)} {...rest} />;
}
