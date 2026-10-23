// What Teach opens: the pull request panel's side sheet, over the Session it was pressed in, so the thread stays in view
// behind it. The words and the owner's instruction are read by Fleet, shown back, and kept only on Save. **While the
// understanding is shown, a link rule is already drawn in the thread** as a preview, and it goes if the panel closes or Change is pressed.

import { useEffect, useRef, useState } from "react";
import { Sheet, TeachPanel, forgetKept } from "@armada/components";
import type { TeachReach } from "@armada/components";
import { useAtFloor } from "@armada/shell";

import { previewLesson, readTeaching, saveLesson } from "../teach";
import type { TeachAnswer } from "../teach";
import { closePanel } from "./store";
import type { PanelItem } from "./store";

type Stage = "writing" | "reading" | "shown";

export function TeachSide({ item }: { item: Extract<PanelItem, { kind: "teach" }> }) {
  const floor = useAtFloor();
  const keep = `teach:${item.sessionId}`;
  const [say, setSay] = useState("");
  const [reach, setReach] = useState<TeachReach>("repository");
  const [stage, setStage] = useState<Stage>("writing");
  const [answer, setAnswer] = useState<TeachAnswer | undefined>(undefined);
  // Each read takes a number; an answer to an older one, or to a panel since closed, is not drawn.
  const latest = useRef(0);
  useEffect(() => () => void (latest.current += 1), []);

  useEffect(() => {
    previewLesson(
      stage === "shown" && answer?.link !== undefined
        ? { id: "preview", reach, repository: item.repository, link: answer.link, ...(item.manifestId === undefined ? {} : { manifestId: item.manifestId }) }
        : undefined,
    );
    return () => previewLesson(undefined);
  }, [stage, answer, reach, item.repository, item.manifestId]);

  const teach = () => {
    const mine = (latest.current += 1);
    setStage("reading");
    void readTeaching({ words: item.words, say, reach, repository: item.repository }).then((read) => {
      if (mine !== latest.current) return;
      setAnswer(read);
      setStage("shown");
    });
  };
  const save = () => {
    if (answer === undefined) return;
    saveLesson({
      reach,
      repository: item.repository,
      ...(item.manifestId === undefined ? {} : { manifestId: item.manifestId }),
      ...(answer.link === undefined ? {} : { link: answer.link }),
      ...(answer.note === undefined ? {} : { note: answer.note }),
    });
    // What was written is now taught, and the box starts empty the next time.
    forgetKept(keep);
    closePanel();
  };

  return (
    <Sheet kind="side-panel" open floating through floor={floor} title="Teach" subtitle={<span className="armada-pr-panel__repository mono">{item.repository}</span>} size="panel" closeLabel="Close" closeBinding="Esc" onClose={closePanel}>
      <TeachPanel
        words={item.words}
        say={say}
        onSay={setSay}
        keep={keep}
        repository={item.repository}
        reach={reach}
        onReach={setReach}
        stage={stage}
        reading={answer}
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
