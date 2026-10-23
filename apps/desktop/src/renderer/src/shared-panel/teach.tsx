// What Teach opens: the pull request panel's side sheet, over the Session it was pressed in, so the thread stays in view
// behind it. What Teach was pressed on and the owner's instruction go to Fleet (`readTeaching`), which reads them into a proposal, shown
// back as a section for each of Agents, Bridge and Fleet that the teaching changes, and kept only on Save (`saveTeaching`). **While the
// understanding is shown, a link rule is already drawn in the thread, and a theme tweak is already on the screen**, as a preview, and each
// goes if the panel closes or Change is pressed. A refusal of either call is raised as the window's toast, in Fleet's words, and the
// panel stays where it was. Pressed on Bridge's own screen, the panel has no repository to reach and no note for agents.

import { useEffect, useRef, useState } from "react";
import { Sheet, TeachPanel, forgetKept, isBridgeSource } from "@armada/components";
import type { TeachReach } from "@armada/components";
import type { TeachingProposal } from "@armada/protocol";
import { useAtFloor } from "@armada/shell";

import { previewTeaching, teachFlavour, teachingSaved } from "../teach";
import { previewOf, readBody, readingOf, saveBody } from "../teach-wire";
import { rememberTaught } from "../taught-entries";
import { raiseOutcome } from "../tell";
import { closePanel } from "./store";
import type { PanelItem } from "./store";

type Stage = "writing" | "reading" | "shown";

export function TeachSide({ item }: { item: Extract<PanelItem, { kind: "teach" }> }) {
  const floor = useAtFloor();
  const keep = `teach:${item.sessionId}`;
  const [say, setSay] = useState("");
  const onBridge = isBridgeSource(item.source);
  const [reach, setReach] = useState<TeachReach>(onBridge ? "all" : "repository");
  const [stage, setStage] = useState<Stage>("writing");
  const [proposal, setProposal] = useState<TeachingProposal | undefined>(undefined);
  const saving = useRef(false);
  // Each read takes a number; an answer to an older one, or to a panel since closed, is not drawn.
  const latest = useRef(0);
  useEffect(() => () => void (latest.current += 1), []);

  const flavour = teachFlavour();
  const reading = proposal === undefined ? undefined : (flavour?.reading?.(proposal, item.repository) ?? readingOf(proposal, item.repository));

  useEffect(() => {
    previewTeaching(stage === "shown" && proposal !== undefined ? previewOf(proposal, reach, item.manifestId, flavour?.link?.(proposal)) : undefined);
    return () => previewTeaching(undefined);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- `flavour` is the window's, set before any panel opens.
  }, [stage, proposal, reach, item]);

  const teach = () => {
    const mine = (latest.current += 1);
    setStage("reading");
    void window.armada.readTeaching(readBody(item, say, reach)).then((read) => {
      if (mine !== latest.current) return;
      if (!read.ok) {
        // Fleet's refusal is the toast, and the box keeps what was written.
        raiseOutcome(read.outcome);
        setStage("writing");
        return;
      }
      setProposal(read.value);
      setStage("shown");
    });
  };
  const save = () => {
    if (proposal === undefined || saving.current) return;
    saving.current = true;
    void window.armada.saveTeaching(saveBody(item, proposal, reach)).then((saved) => {
      saving.current = false;
      if (!saved.ok) return raiseOutcome(saved.outcome);
      rememberTaught(saved.value.entry, saved.value.mods);
      teachingSaved();
      // What was written is now taught, and the box starts empty the next time.
      forgetKept(keep);
      closePanel();
    });
  };

  return (
    <Sheet kind="side-panel" open floating through floor={floor} title="Teach" {...(onBridge ? {} : { subtitle: <span className="armada-pr-panel__repository mono">{item.repository}</span> })} size="panel" closeLabel="Close" closeBinding="Esc" onClose={closePanel}>
      <TeachPanel
        source={{ kind: item.source, text: item.text }}
        say={say}
        onSay={setSay}
        keep={keep}
        repository={item.repository}
        reach={reach}
        onReach={setReach}
        stage={stage}
        reading={reading}
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
