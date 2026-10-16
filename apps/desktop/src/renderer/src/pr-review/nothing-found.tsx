// What a review tool shows when it looked and found nothing: a small picture, drawn in the border tokens and tinted a little
// with the tool's hue. The owner's second illustration, after the merge line's (`EmptyLine`, `iconography.md`, Where the answer
// is no icon). Its name is for a screen reader alone. One line may sit under it, and only one that names what was checked.

import type { ToolMark } from "./model";

/** What a tool looks for, as the line under the picture says it. A tool of his own has no noun, so it says nothing. */
const LOOKED_FOR: Partial<Record<ToolMark, string>> = { risks: "risks", bugs: "possible bugs", breaks: "API breaks" };

/** The picture, and under it what was checked: `No risks in 6 changes`. Nothing under it where the tool has no noun or there were no changes. */
export function NothingFound({ mark, changes }: { mark: ToolMark; changes: number }) {
  const noun = LOOKED_FOR[mark];
  return (
    <div className="pr-nothing">
      <svg className="pr-nothing__picture" viewBox="0 0 96 40" role="img" aria-label="Nothing found">
        <path className="pr-nothing__line" d="M8 10 H44" />
        <path className="pr-nothing__line" d="M8 20 H36" />
        <path className="pr-nothing__line" d="M8 30 H48" />
        <circle className="pr-nothing__lens" cx="66" cy="17" r="11" />
        <path className="pr-nothing__handle" d="M74 25 L84 35" />
      </svg>
      {noun === undefined || changes === 0 ? null : <p className="pr-nothing__said">{`No ${noun} in ${changes} ${changes === 1 ? "change" : "changes"}`}</p>}
    </div>
  );
}
