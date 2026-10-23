import { useEffect, useRef } from "react";
import type { KeyboardEvent } from "react";
import { LoaderCircle } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { KbdChord } from "../../primitives/Kbd/Kbd";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/** Where a lesson applies: the repository the Session is in, or all of them. */
export type TeachReach = "repository" | "all";

/**
 * What Fleet understood of the person's words. **Either part, or both**: a link rule makes matching words links in every
 * Session, an agent note goes to agents. `address` writes `{match}` where the matched words go.
 */
export type TeachReading = {
  link?: { example: string; address: string };
  note?: string;
};

export type TeachPanelProps = {
  /** The words selected in the message, as they stand there. */
  words: string;
  /** What the person said they are and what Armada should do. */
  say: string;
  onSay: (say: string) => void;
  /** Names the textarea's draft, so it survives leaving the panel. */
  keep: string;
  /** The repository's name, on the chip while the lesson is for this repository. */
  repository: string;
  reach: TeachReach;
  onReach: (reach: TeachReach) => void;
  /** `writing`: the words and the box. `reading`: Fleet is reading them. `shown`: what it understood, to save or change. */
  stage: "writing" | "reading" | "shown";
  /** What Fleet understood. Drawn in `shown`. */
  reading?: TeachReading | undefined;
  onTeach: () => void;
  /** Back to the box, with the words kept. */
  onChange: () => void;
  onSave: () => void;
};

/**
 * The body of the panel Teach opens: the selected words, a box for what to do with them, and the reach. **Armada shows
 * back what it understood before it keeps anything**: the link rule as an example and its address, the note agents will
 * be given, and Change or Save. ⌘↵ is the forward key in both steps; the stage's one primary button wears it.
 */
export function TeachPanel({ words, say, onSay, keep, repository, reach, onReach, stage, reading, onTeach, onChange, onSave }: TeachPanelProps) {
  const body = useRef<HTMLDivElement>(null);
  // A sheet puts focus on its close as it opens. The box, or the stage's forward button, takes it a beat later.
  useEffect(() => {
    const timer = window.setTimeout(() => body.current?.querySelector<HTMLElement>(stage === "writing" ? "textarea" : stage === "shown" ? "[data-forward]" : "button")?.focus(), 0);
    return () => window.clearTimeout(timer);
  }, [stage]);
  const ready = say.trim() !== "";
  const forward = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== "Enter" || !(event.metaKey || event.ctrlKey)) return;
    event.preventDefault();
    if (stage === "writing" && ready) onTeach();
    else if (stage === "shown") onSave();
  };
  const everywhere = reach === "all";
  return (
    <div className="armada-teach-panel" ref={body} onKeyDown={forward}>
      <section className="armada-teach-panel__section" aria-label="Words">
        <h4 className="armada-teach-panel__caption">Words</h4>
        <p className="armada-teach-panel__words mono">{words}</p>
      </section>

      {stage === "writing" ? (
        <Textarea label="Instruction" rows={4} grow keep={keep} value={say} onChange={(event) => onSay(event.target.value)} />
      ) : (
        <section className="armada-teach-panel__section" aria-label="Instruction">
          <h4 className="armada-teach-panel__caption">Instruction</h4>
          <p className="armada-teach-panel__said">{say}</p>
        </section>
      )}

      <div className="armada-teach-panel__reach">
        <Tooltip label="Where it applies">
          <button type="button" className="armada-teach-panel__chip" aria-pressed={everywhere} disabled={stage !== "writing"} onClick={() => onReach(everywhere ? "repository" : "all")}>
            <span className={everywhere ? undefined : "mono"}>{everywhere ? "Every repository" : repository}</span>
          </button>
        </Tooltip>
      </div>

      {stage === "writing" ? (
        <div className="armada-teach-panel__acts">
          <Button variant="primary" disabled={!ready} onClick={onTeach} aria-keyshortcuts="Meta+Enter">
            Teach
            <KbdChord keys={["⌘", "↵"]} aria-hidden />
          </Button>
        </div>
      ) : null}

      {stage === "reading" ? (
        <Tooltip label="Reading what you wrote">
          <span className="armada-teach-panel__reading" role="status" aria-label="Reading what you wrote">
            <LoaderCircle size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
      ) : null}

      {stage === "shown" ? (
        <>
          {reading?.link === undefined ? null : (
            <section className="armada-teach-panel__section" aria-label="Link">
              <h4 className="armada-teach-panel__caption">Link</h4>
              <p className="armada-teach-panel__rule">
                <span className="mono">{reading.link.example}</span>
                <span className="armada-teach-panel__address mono">{reading.link.address.replaceAll("{match}", reading.link.example)}</span>
              </p>
            </section>
          )}
          {reading?.note === undefined ? null : (
            <section className="armada-teach-panel__section" aria-label="Agents">
              <h4 className="armada-teach-panel__caption">Agents</h4>
              <p className="armada-teach-panel__said">{reading.note}</p>
            </section>
          )}
          <div className="armada-teach-panel__acts">
            <Button variant="secondary" onClick={onChange}>
              Change
            </Button>
            <Button variant="primary" data-forward="" onClick={onSave} aria-keyshortcuts="Meta+Enter">
              Save
              <KbdChord keys={["⌘", "↵"]} aria-hidden />
            </Button>
          </div>
        </>
      ) : null}
    </div>
  );
}
