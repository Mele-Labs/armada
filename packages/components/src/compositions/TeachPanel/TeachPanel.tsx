import { useEffect, useRef } from "react";
import type { KeyboardEvent } from "react";
import { LoaderCircle } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { KbdChord } from "../../primitives/Kbd/Kbd";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/** Where a lesson applies: the repository the Session is in, or all of them. */
export type TeachReach = "repository" | "all";

/** What Teach was pressed on. */
export type TeachSourceKind = "words" | "command" | "notice" | "task" | "check" | "pull_request";

/** The name of each, as the panel's first section captions it. */
export const TEACH_SOURCE: Record<TeachSourceKind, string> = {
  words: "Words",
  command: "Command",
  notice: "Notice",
  task: "Task",
  check: "Check",
  pull_request: "Pull request",
};

/**
 * What Fleet understood of the person's words. **Any of the three, and only those that apply**: `agents` is the note
 * agents are handed, `bridge` is how Bridge draws words that match a pattern, `fleet` is what Fleet does with a notice.
 * Every string is drawn as it is given.
 */
export type TeachReading = {
  agents?: string;
  bridge?: {
    /** What the pattern finds, as the person would name it. */
    name: string;
    pattern: string;
    /** One thing the pattern found, and the address it goes to in this repository. Absent while none is found. */
    example: string;
    address?: string;
    /** Where the address came from. */
    found?: string;
  };
  fleet?: { notice: string; does: string; scope: string };
};

export type TeachPanelProps = {
  /** What Teach was pressed on, and its text as it stands there. */
  source: { kind: TeachSourceKind; text: string };
  /** What the person said they are and what Armada should do. */
  say: string;
  onSay: (say: string) => void;
  /** Names the textarea's draft, so it survives leaving the panel. */
  keep: string;
  /** The repository's name, on the chip while the lesson is for this repository. */
  repository: string;
  reach: TeachReach;
  onReach: (reach: TeachReach) => void;
  /** `writing`: the source and the box. `reading`: Fleet is reading them. `shown`: what it understood, to save or change. */
  stage: "writing" | "reading" | "shown";
  /** What Fleet understood. Drawn in `shown`. */
  reading?: TeachReading | undefined;
  onTeach: () => void;
  /** Back to the box, with the source kept. */
  onChange: () => void;
  onSave: () => void;
};

/**
 * The body of the panel Teach opens: what it was pressed on, a box for what to do about it, and the reach. **Armada shows
 * back what it understood before it keeps anything**, in a section for each of Agents, Bridge and Fleet that the teaching
 * changes, then Change or Save. ⌘↵ is the forward key in both steps; the stage's one primary button wears it.
 */
export function TeachPanel({ source, say, onSay, keep, repository, reach, onReach, stage, reading, onTeach, onChange, onSave }: TeachPanelProps) {
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
  const caption = TEACH_SOURCE[source.kind];
  return (
    <div className="armada-teach-panel" ref={body} onKeyDown={forward}>
      <section className="armada-teach-panel__section" aria-label={caption}>
        <h4 className="armada-teach-panel__caption">{caption}</h4>
        <p className={`armada-teach-panel__words${source.kind === "words" || source.kind === "notice" || source.kind === "task" ? "" : " mono"}`}>{source.text}</p>
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
          {reading?.agents === undefined ? null : (
            <section className="armada-teach-panel__section" aria-label="Agents">
              <h4 className="armada-teach-panel__caption">Agents</h4>
              <p className="armada-teach-panel__said">{reading.agents}</p>
            </section>
          )}
          {reading?.bridge === undefined ? null : (
            <section className="armada-teach-panel__section" aria-label="Bridge">
              <h4 className="armada-teach-panel__caption">Bridge</h4>
              <dl className="armada-teach-panel__facts">
                <dt>{reading.bridge.name}</dt>
                <dd className="mono">{reading.bridge.pattern}</dd>
                {reading.bridge.address === undefined ? null : (
                  <>
                    <dt className="mono">{reading.bridge.example}</dt>
                    <dd className="mono">{reading.bridge.address}</dd>
                    {reading.bridge.found === undefined ? null : <dd className="armada-teach-panel__found">{reading.bridge.found}</dd>}
                  </>
                )}
              </dl>
            </section>
          )}
          {reading?.fleet === undefined ? null : (
            <section className="armada-teach-panel__section" aria-label="Fleet">
              <h4 className="armada-teach-panel__caption">Fleet</h4>
              <dl className="armada-teach-panel__facts">
                <dt>Notice</dt>
                <dd>{reading.fleet.notice}</dd>
                <dt>Does</dt>
                <dd>{reading.fleet.does}</dd>
                <dt>Applies to</dt>
                <dd>{reading.fleet.scope}</dd>
              </dl>
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
