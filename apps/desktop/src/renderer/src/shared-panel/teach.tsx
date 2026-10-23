// What Teach opens: the pull request panel's side sheet, over the Session it was pressed in, so the thread stays in view
// behind it. What Teach was pressed on and the owner's instruction are read by Fleet, shown back as a section for each of Agents,
// Bridge and Fleet that the teaching changes, and kept only on Save. **While the understanding is shown, a link rule is already
// drawn in the thread** as a preview, and it goes if the panel closes or Change is pressed.

import { useEffect, useRef, useState } from "react";
import { Sheet, TeachPanel, forgetKept } from "@armada/components";
import type { TeachReach, TeachReading } from "@armada/components";
import { useAtFloor } from "@armada/shell";

import { addressOf, previewLesson, readTeaching, saveLesson, siteSaid } from "../teach";
import type { Lesson, TeachAnswer, Teaching } from "../teach";
import { closePanel } from "./store";
import type { PanelItem } from "./store";

type Stage = "writing" | "reading" | "shown";

/** What Fleet understood, as the panel words it. */
function readingOf(answer: TeachAnswer, repository: string): TeachReading {
  const { agents, bridge, fleet } = answer;
  return {
    ...(agents === undefined ? {} : { agents }),
    ...(bridge === undefined
      ? {}
      : {
          bridge: {
            name: bridge.name,
            pattern: bridge.pattern,
            example: bridge.example,
            ...(bridge.site === undefined ? {} : { address: addressOf(bridge.site, bridge.example), found: siteSaid(bridge.site.from) }),
          },
        }),
    ...(fleet === undefined ? {} : { fleet: { notice: fleet.name, does: fleet.says, scope: fleet.scope === "job" ? "This Job" : fleet.scope === "all" ? "Every repository" : repository } }),
  };
}

export function TeachSide({ item }: { item: Extract<PanelItem, { kind: "teach" }> }) {
  const floor = useAtFloor();
  const keep = `teach:${item.sessionId}`;
  const [say, setSay] = useState("");
  const [reach, setReach] = useState<TeachReach>("repository");
  const [stage, setStage] = useState<Stage>("writing");
  const [answer, setAnswer] = useState<TeachAnswer | undefined>(undefined);
  const teaching: Teaching = {
    kind: item.source,
    text: item.text,
    ...(item.at === undefined ? {} : { at: item.at }),
    sessionId: item.sessionId,
    jobTitle: item.jobTitle,
    ...(item.manifestId === undefined ? {} : { manifestId: item.manifestId }),
    ...(item.root === undefined ? {} : { root: item.root }),
    repository: item.repository,
  };
  // Each read takes a number; an answer to an older one, or to a panel since closed, is not drawn.
  const latest = useRef(0);
  useEffect(() => () => void (latest.current += 1), []);

  useEffect(() => {
    previewLesson(
      stage === "shown" && answer?.bridge !== undefined ? { id: "preview", reach, teaching, say, answer, at: "" } satisfies Lesson : undefined,
    );
    return () => previewLesson(undefined);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- `teaching` is rebuilt from `item` on every render.
  }, [stage, answer, reach, say, item]);

  const teach = () => {
    const mine = (latest.current += 1);
    setStage("reading");
    void readTeaching({ teaching, say, reach }).then((read) => {
      if (mine !== latest.current) return;
      setAnswer(read);
      setStage("shown");
    });
  };
  const save = () => {
    if (answer === undefined) return;
    saveLesson({ reach, teaching, say, answer });
    // What was written is now taught, and the box starts empty the next time.
    forgetKept(keep);
    closePanel();
  };

  return (
    <Sheet kind="side-panel" open floating through floor={floor} title="Teach" subtitle={<span className="armada-pr-panel__repository mono">{item.repository}</span>} size="panel" closeLabel="Close" closeBinding="Esc" onClose={closePanel}>
      <TeachPanel
        source={{ kind: item.source, text: item.text }}
        say={say}
        onSay={setSay}
        keep={keep}
        repository={item.repository}
        reach={reach}
        onReach={setReach}
        stage={stage}
        reading={answer === undefined ? undefined : readingOf(answer, item.repository)}
        onTeach={teach}
        onChange={() => {
          latest.current += 1;
          setStage("writing");
        }}
        onSave={save}
      />
    </Sheet>
  );
}
