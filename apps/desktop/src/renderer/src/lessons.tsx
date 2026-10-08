// The Lessons surface, as the window draws it: what got in the way across
// Jobs, read on open and on focus (`docs/concepts/retro.md`). Apart from
// `App.tsx`, which is at its length.

import { useState } from "react";

import { Lessons } from "@armada/jobs";
import { Boundary, useAtFloor } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import { agreeLesson, disagreeLesson, readLessons, readRetro } from "./commands";
import { takeRetroAsked } from "./open-retro";
import { useLessonsTab } from "./remembered-views";

export function LessonsSurface({
  repository,
  bridge,
  onCopied,
  onOpenJob,
}: {
  repository: string | null;
  /** A Job an agreed item proposed opens over Lessons, and Escape comes back here. */
  onOpenJob: (jobId: string) => void;
  bridge: BridgeState["bridge"];
  onCopied: (value: string) => void;
}) {
  const floor = useAtFloor();
  // The tab is remembered for this viewer, the way Workflow's and Plan's views are.
  const [tab, setTab] = useLessonsTab();
  // A Session's Retro press opens the page on that Session's retro; the ask is taken once.
  const [asked] = useState(takeRetroAsked);
  return (
    <Boundary region="Retros" bridge={bridge} onCopied={onCopied}>
      {/* Keyed by the pick, because main narrows the read to it. */}
      <Lessons
        key={repository ?? ""}
        onReadLessons={readLessons}
        onReadRetro={readRetro}
        onAgreeLesson={agreeLesson}
        onDisagreeLesson={disagreeLesson}
        onOpenJob={onOpenJob}
        repository={repository}
        floor={floor}
        tab={tab}
        onTab={setTab}
        {...(asked === undefined ? {} : { opening: asked })}
      />
    </Boundary>
  );
}
