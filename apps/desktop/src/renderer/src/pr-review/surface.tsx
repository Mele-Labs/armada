// The Reviews page: the review surface in a frame of its own. The list is the left column and the reader the stage, in the
// Cockpit's own grid. The review tools are tabs of the reader's panes.

import { useRef } from "react";
import { DockHandle } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";

import { useColumns } from "../cockpit/deck/columns";
import { ReviewList } from "./list";
import { ReviewReader } from "./reader";
import { SpendConfirm } from "./flight";
import { ReviewKeys } from "./keys";
import { WalkSheets } from "./walk";
import { PickedRepository } from "./piece-root";
import "../cockpit/cockpit.css";
import "../cockpit/deck/deck.css";
import "./pr-review.css";

export function ReviewsSurface({
  picked,
  repositories,
  onOpenLink,
  onOpenSession,
}: {
  picked: RepositorySummary | null;
  repositories: readonly RepositorySummary[];
  onOpenLink: (address: string) => void;
  onOpenSession: (id: string) => void;
}) {
  const grid = useRef<HTMLDivElement>(null);
  const { columns, resize, reset } = useColumns(grid, "reviews");
  return (
    <PickedRepository.Provider value={picked}>
      <div className="armada-cockpit deck pr-surface">
        <div
          ref={grid}
          className="deck-grid"
          data-review
          style={
            columns === undefined
              ? undefined
              : ({
                  "--deck-left": `${columns.left}px`,
                  "--deck-right": `${columns.right}px`,
                } as React.CSSProperties)
          }
        >
          <div className="deck-left">
            <ReviewList picked={picked} repositories={repositories} />
            {columns === undefined ? null : (
              <DockHandle
                quiet
                className="deck-handle"
                grows="right"
                label="the left column"
                width={columns.left}
                {...columns.range.left}
                onResize={(width) => resize("left", width)}
                onReset={() => reset("left")}
              />
            )}
          </div>
          <div className="deck-centre">
            <div className="deck-stage">
              <ReviewReader onOpenLink={onOpenLink} onOpenSession={onOpenSession} />
            </div>
            <SpendConfirm />
            <WalkSheets />
            <ReviewKeys />
          </div>
        </div>
      </div>
    </PickedRepository.Provider>
  );
}
