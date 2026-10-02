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
  selected?: boolean;
};

/** What a frame is read aloud as: its kind, then its title where it has one. */
export function studioFrameLabel({ kind, title }: Pick<StudioFrameProps, "kind" | "title">): string {
  return title === undefined || title === "" ? STUDIO_NODE_KIND[kind] : `${STUDIO_NODE_KIND[kind]}: ${title}`;
}

export function StudioFrame({ kind, title, selected = false }: StudioFrameProps) {
  return (
    <div className="armada-studio-frame" data-kind={kind} data-selected={selected || undefined}>
      <div className="armada-studio-frame__head">
        <span className="armada-studio-frame__kind">{STUDIO_NODE_KIND[kind]}</span>
        {title === undefined || title === "" ? null : (
          <span className="armada-studio-frame__title" title={title}>
            {title}
          </span>
        )}
      </div>
    </div>
  );
}
