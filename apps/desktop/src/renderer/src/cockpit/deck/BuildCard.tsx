// What a Session's card says of its construction: the phase, its plan as one bar of a segment per task as its piece stands on the
// field, and the road's stops. A segment is drawn as its piece is (solid, outlined, open) with the task and its state as its tooltip.
// No count is drawn beside the items it counts. #2301.

import { Tooltip } from "@armada/components";

import type { BuildTip, Piece } from "./build";

/** What a piece says of its task. */
export const SAID: Record<Piece, string> = { solid: "Done, edits seen", hollow: "Done, no edits seen", scaffold: "Open, edits seen", ghost: "Open" };

export function BuildCard({ build }: { build: BuildTip }) {
  return (
    <div className="bf-build">
      <div className="bf-build__phase">
        <Tooltip label="Phase" asChild>
          <span className="bf-build__glyph">
            <build.phase.Glyph size={16} strokeWidth={2} aria-label="Phase" />
          </span>
        </Tooltip>
        {build.phase.word}
        {build.quiet === undefined ? null : <span className="bf-build__quiet">{build.quiet}</span>}
      </div>
      {build.tasks.length === 0 ? null : (
        // The plan as one bar, a segment per task in order; the task and its state are the segment's tooltip (the owner, 17 Oct 2026).
        <ul className="bf-build__plan" aria-label="Plan">
          {build.tasks.map((task, i) => (
            <Tooltip key={i} label={`${task.text} · ${SAID[task.piece]}`} asChild>
              <li className="bf-build__seg" data-piece={task.piece} aria-label={`${task.text}, ${SAID[task.piece]}`} />
            </Tooltip>
          ))}
        </ul>
      )}
      {build.stops.length === 0 ? null : (
        <ul className="bf-build__stops">
          {build.stops.map((stop) => (
            <li key={stop.word} className="bf-build__stop" data-faint={stop.ago === undefined || undefined}>
              <stop.Glyph size={14} strokeWidth={2} aria-hidden="true" />
              <span>{stop.word}</span>
              {stop.ago === undefined ? null : <time>{stop.ago}</time>}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
