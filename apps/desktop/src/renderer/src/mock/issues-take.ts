// The owner's three takes on Progress's issues block (`?take=objectives|strips|berths`); with none, the plain list. Mock only: the take he
// chooses becomes the block's drawing and this file goes. The take also draws the pull requests a plan group holds.

import { takeIssuesDrawing, takePullRequestsDrawing } from "../session-view/IssuesBlock";
import { Berths, BerthsPrs, Objectives, ObjectivesPrs, Strips, StripsPrs } from "../session-view/IssuesTakes";

const TAKES = { objectives: [Objectives, ObjectivesPrs], strips: [Strips, StripsPrs], berths: [Berths, BerthsPrs] } as const;

export function takeTheIssuesDrawing(take: string | null): void {
  const drawing = take === null ? undefined : TAKES[take as keyof typeof TAKES];
  if (drawing === undefined) return;
  takeIssuesDrawing(drawing[0]);
  takePullRequestsDrawing(drawing[1]);
}
