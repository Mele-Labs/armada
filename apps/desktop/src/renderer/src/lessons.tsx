// The Lessons surface, as the window draws it: what got in the way across
// Jobs, read on open and on focus (`docs/concepts/retro.md`). Apart from
// `App.tsx`, which is at its length.

import { Lessons } from "@armada/screens";
import { Boundary, useAtFloor } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import { readLessons, readRetro } from "./commands";
import { useLessonsTab } from "./remembered-views";

export function LessonsSurface({
  repository,
  bridge,
  onCopied,
}: {
  repository: string | null;
  bridge: BridgeState["bridge"];
  onCopied: (value: string) => void;
}) {
  const floor = useAtFloor();
  // The tab is remembered for this viewer, the way Workflow's and Plan's views are.
  const [tab, setTab] = useLessonsTab();
  return (
    <Boundary region="Lessons" bridge={bridge} onCopied={onCopied}>
      {/* Keyed by the pick, because main narrows the read to it. */}
      <Lessons
        key={repository ?? ""}
        onReadLessons={readLessons}
        onReadRetro={readRetro}
        repository={repository}
        floor={floor}
        tab={tab}
        onTab={setTab}
      />
    </Boundary>
  );
}
