// The Logbook's Retros tab: every retro item, newest first, read on open and on focus
// (`docs/concepts/retro.md`). Apart from `App.tsx`, which is at its length.

import { useState } from "react";

import { createReviewHold } from "@armada/screens/src/lessons/review-hold";
import { Lessons } from "@armada/screens/src/lessons/Lessons";
import { useAtFloor } from "@armada/shell";

import { agreeLesson, askLesson, disagreeLesson, readLessons, readRetro, reviewLessons, startLessonSession } from "./commands";
import { takeRetroAsked } from "./open-retro";
import { askToOpenSession } from "./open-session";

/**
 * The guided review's held state for this window, per repository pick: made once, outside the
 * surface, because the surface unmounts whenever the rail goes elsewhere. Gone with the window.
 */
const reviewHold = createReviewHold();

export function ArmadaFixes({ repository }: { repository: string | null }) {
  const floor = useAtFloor();
  // A Session's Retro press opens the page on that Session's retro; the ask is taken once.
  const [asked] = useState(takeRetroAsked);
  return (
    // Keyed by the pick, because main narrows the read to it.
    <Lessons
      key={repository ?? ""}
      embedded
      onReadLessons={readLessons}
      onReadRetro={readRetro}
      onAgreeLesson={agreeLesson}
      onDisagreeLesson={disagreeLesson}
      onStartSession={startLessonSession}
      onOpenSession={askToOpenSession}
      onReviewLessons={reviewLessons}
      onAskLesson={askLesson}
      repository={repository}
      floor={floor}
      hold={reviewHold}
      {...(asked === undefined ? {} : { opening: asked })}
    />
  );
}
