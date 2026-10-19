// The owner's two takes on the Draft mark (`?take=a|b`): A is `message-square-dashed`, which production
// draws, and B is `text-cursor-input`. Mock only; the take he chooses replaces this file.

import { TextCursorInput } from "lucide-react";

import { takeDraftGlyph } from "@armada/components";

export function takeTheDraftMark(take: string | null): void {
  if (take === "b") takeDraftGlyph(TextCursorInput);
}
