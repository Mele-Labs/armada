// What a Session's card says of its construction: the phase, one row for each task as its piece stands on the field, and the road's
// stops. A task is a swatch drawn as its piece is (solid, outlined, open) with the fact behind it as its tooltip. No count is drawn
// beside the items it counts. #2301.

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
        <ul className="bf-build__tasks">
          {build.tasks.map((task, i) => (
            <li key={i} className="bf-build__task" data-piece={task.piece}>
              <Tooltip label={SAID[task.piece]} asChild>
                <i className="bf-build__swatch" role="img" aria-label={SAID[task.piece]} />
              </Tooltip>
              <span>{task.text}</span>
            </li>
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
