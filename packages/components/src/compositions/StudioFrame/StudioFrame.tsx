import type { ReactNode } from "react";

import { STUDIO_NODE_KIND } from "../StudioNode/StudioNode";

/**
 * Studio frame — a node on a Studio's whiteboard that holds others: a Zone,
 * or a Cluster round its Notes. `#1620`, decided with the owner on 2 Oct 2026.
 *
 * **A box and its head, and nothing inside.** What a frame holds are nodes of
 * their own, drawn by the whiteboard inside its bounds; this draws the region
 * they sit in and fills whatever size the whiteboard gives it.
 *
 * **A Zone is a well and a Cluster is a dashed box**, so a Cluster sitting in
 * a read-in's Zone reads as a group inside a region rather than a second
 * region. A Cluster's head carries its title; a Zone holds no words, so its
 * head is its kind alone.
 */

export type StudioFrameKind = "zone" | "cluster";

export type StudioFrameProps = {
  kind: StudioFrameKind;
  /** A Cluster's title. A Zone has none, and draws none. */
  title?: string;
  /** The head's word in the kind's place, where the kind alone says nothing — a canvas lane. Absent is the kind. */
  name?: string;
  /** What the head draws in the kind's place, where it is a control — a canvas lane's picker. Wins over `name`. */
  head?: ReactNode;
  selected?: boolean;
  /** Where the work inside is, on a run's canvas: `live` is under way, `done` recedes. Absent is the frame as it was. */
  tone?: "live" | "done";
};

/** What a frame is read aloud as: its kind, then its title where it has one. */
export function studioFrameLabel({ kind, title }: Pick<StudioFrameProps, "kind" | "title">): string {
  return title === undefined || title === "" ? STUDIO_NODE_KIND[kind] : `${STUDIO_NODE_KIND[kind]}: ${title}`;
}

export function StudioFrame({ kind, title, name, head, selected = false, tone }: StudioFrameProps) {
  return (
    <div className="armada-studio-frame" data-kind={kind} data-selected={selected || undefined} data-tone={tone}>
      <div className="armada-studio-frame__head">
        {head ?? <span className="armada-studio-frame__kind">{name ?? STUDIO_NODE_KIND[kind]}</span>}
        {title === undefined || title === "" ? null : (
          <span className="armada-studio-frame__title" title={title}>
            {title}
          </span>
        )}
      </div>
    </div>
  );
}
