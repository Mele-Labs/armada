// The owner's three takes on Progress's issues block (`?take=objectives|strips|berths`); with none, the plain list. Mock only: the take he
// chooses becomes the block's drawing and this file goes.

import { takeIssuesDrawing } from "../session-view/IssuesBlock";
import { Berths, Objectives, Strips } from "../session-view/IssuesTakes";

const TAKES = { objectives: Objectives, strips: Strips, berths: Berths } as const;

export function takeTheIssuesDrawing(take: string | null): void {
  const drawing = take === null ? undefined : TAKES[take as keyof typeof TAKES];
  if (drawing !== undefined) takeIssuesDrawing(drawing);
}
