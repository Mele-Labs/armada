import { Compass, PanelRightClose } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { SessionComposer } from "../SessionComposer/SessionComposer";
import type { ComposerTag } from "../SessionComposer/SessionComposer";
import { SessionMark } from "../SessionFrame/SessionFrame";
import type { SessionState } from "../SessionFrame/SessionFrame";
import { SessionThread } from "../SessionThread/SessionThread";
import type { SessionThreadRow } from "../SessionThread/SessionThread";

/**
 * A Job's Commander, beside its canvas: the one agent conversation the Job has, drawn on a
 * Session's thread (`SessionThread`) under a band that carries its state.
 *
 * **State is the frame**, as the Now panel's bands are: the band takes the running hue while a turn
 * is out, the escalated hue while the Commander waits on the owner, and the quiet hue between. The
 * mark beside the title is the Session mark set, with a tooltip. Fleet's wake, the Commander's own
 * reads and what it did on its own are rows of the thread; the owner's words are rows of it too, so
 * asking and reading the answer is one loop.
 *
 * **The box is the Sessions box** (`SessionComposer`), text only: ↵ sends and ⇧↵ breaks the line, `@`
 * writes a tag into the message, and what is typed is kept per Job until it is sent. It offers
 * no model, effort, permission mode, file or sketch, which the Commander cannot take.
 *
 * **No empty sentence.** A Commander that has done nothing draws the band and the box.
 */
export type CommanderState = "idle" | "working" | "waiting";

export type CommanderPanelProps = {
  /** Which Job it is the Commander of: what is typed is kept under it until it is sent. */
  jobId: string;
  state: CommanderState;
  rows: readonly SessionThreadRow[];
  /** What the owner typed, sent as the Commander's next input. */
  onSend: (text: string) => void;
  /** What `@` offers. Absent, the box offers none. */
  taggable?: readonly ComposerTag[];
  /** The head's hide button. Absent draws none. */
  onHide?: () => void;
};

const STATE: Record<CommanderState, { mark: SessionState; said: string }> = {
  idle: { mark: "idle", said: "Idle" },
  working: { mark: "working", said: "Working" },
  waiting: { mark: "waiting", said: "Waiting on you" },
};

const NOBODY = () => undefined;
const NONE: readonly never[] = [];

export function CommanderPanel({ jobId, state, rows, onSend, taggable = NONE, onHide }: CommanderPanelProps) {
  const { mark, said } = STATE[state];
  return (
    <section className="armada-commander" role="region" aria-label="Commander" data-state={state}>
      <header className="armada-commander__band">
        <Compass size={16} strokeWidth={2} aria-hidden />
        <h3 className="armada-commander__title">Commander</h3>
        <SessionMark state={mark} said={said} />
        {onHide === undefined ? null : (
          <Tooltip label="Hide">
            <Button variant="ghost" size="sm" aria-label="Hide Commander" onClick={onHide}>
              <PanelRightClose size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        )}
      </header>
      <div className="armada-commander__thread">
        <SessionThread
          sessionId={jobId}
          rows={rows}
          working={state === "working"}
          agentName="Commander"
          onAnswer={NOBODY}
          onOpenSession={NOBODY}
        />
      </div>
      <SessionComposer
        // A message to the Commander queues for its next turn, so Send stays on while one runs.
        working={state === "working"}
        textOnly
        label="Tell the Commander"
        send="secondary"
        model={null}
        effort={null}
        models={NONE}
        efforts={NONE}
        onTune={NOBODY}
        commands={NONE}
        taggable={taggable}
        tags={NONE}
        onTags={NOBODY}
        drawn={NONE}
        onDraw={NOBODY}
        onRemoveDrawn={NOBODY}
        draftKey={`commander:${jobId}`}
        onSend={(sent) => onSend(sent.text)}
      />
    </section>
  );
}
