import { useState, type ReactNode } from "react";
import { ExternalLink, Plus, SquarePen, Trash2, Undo2, UserPen } from "lucide-react";

import { forgetKept, useKept } from "../../keep";
import { Button } from "../../primitives/Button/Button";
import { Checkbox } from "../../primitives/Checkbox/Checkbox";
import { Input } from "../../primitives/Input/Input";
import { CopyAct, RunnableBlock, type CodeRunner } from "../../primitives/Prose/Prose";
import { plainOf } from "../../primitives/Prose/plain";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * What a waiting item opens when it is a procedure: the link or the command at the top, the steps numbered
 * as a guide's are with a tick box each, and under them the box the answer is typed in. A step's `backticked`
 * command is set as code and drawn again under it, whole, with its own copy key.
 *
 * **The agent may be wrong about the steps, and the person says so here.** A step is edited in place,
 * dropped (it stays in the list, struck through, so the agent sees what was wrong) or added at the end.
 * Ticks are the person's own place-keeping and nothing depends on them, except that **a command run from its key
 * ticks its step when it exits clean, and one that fails tells the Job** (`onFailed`), with its exit code and last lines. **The send key is never disabled:
 * it reads Done while the reply box is empty and sends "Done", so a procedure can be finished without
 * ticking every step.** **Corrections travel with it**, as plain lines after the reply (`messageOf`).
 * Ticks, corrections and the reply are kept under `keep`, so closing the panel or leaving the Session loses none; a send clears all three.
 */
export type StepsPanelProps = {
  /** The item's words, which name the reply box. */
  name: string;
  /** The page the steps are about, drawn on the key at the top that opens it. */
  address?: string;
  /** The command the steps start from, drawn whole at the top with a key that copies it, unless a step names it. */
  command?: string;
  steps: readonly string[];
  /** What the ticks, the corrections and the draft are about: the Session and the item. */
  keep: string;
  /** Opens the page in an Armada window. */
  onOpenAddress: (address: string) => void;
  /** Opens the page in the person's own browser. */
  onOpenInBrowser: (address: string) => void;
  onSend: (text: string) => void;
  /** Runs a command where the Job works. Left out, a command is copied and not run. */
  runner?: CodeRunner;
  /** Tells the Job a command it asked for failed. */
  onFailed?: (said: string) => void;
};

/** What the person changed: new words by step index, steps dropped, and steps added after the last. */
export type StepFixes = { edited: Record<number, string>; dropped: number[]; added: string[] };

/** How much of a failed run's output goes to the Job. */
const FAILED_LINES = 20;

const NO_TICKS: number[] = [];
const NO_FIXES: StepFixes = { edited: {}, dropped: [], added: [] };

/** What the key sends when the box is empty: the item is finished, whatever was ticked. */
const DONE = "Done";

/** The reply, then one line a correction, in the order of the steps, so the agent reads what was wrong. */
export function messageOf(reply: string, steps: readonly string[], fixes: StepFixes): string {
  const lines = [
    ...steps.flatMap((was, at) => {
      const now = fixes.edited[at];
      return [
        ...(now === undefined ? [] : [`Step ${at + 1} changed: "${was}" → "${now}"`]),
        ...(fixes.dropped.includes(at) ? [`Step ${at + 1} dropped: "${was}"`] : []),
      ];
    }),
    ...fixes.added.map((now, at) => `Added after step ${steps.length + at}: "${now}"`),
  ];
  return [reply.trim(), lines.join("\n")].filter((one) => one !== "").join("\n\n");
}

/** A step's words with each `backticked` span set as code, and the spans themselves: each is a command to copy. */
export function partsOf(line: string): { words: ReactNode[]; commands: string[] } {
  const words: ReactNode[] = [];
  const commands: string[] = [];
  line.split(/`([^`]+)`/).forEach((part, at) => {
    if (at % 2 === 0) {
      if (part !== "") words.push(part);
      return;
    }
    commands.push(part);
    words.push(
      <code key={at} className="armada-step-list__code mono">
        {part}
      </code>,
    );
  });
  return { words, commands };
}

/** The command, wrapped rather than cut, with the keys that run and copy it over the block's corner as a fenced block's are. */
function CommandToCopy({ command, label, runner }: { command: string; label: string; runner?: CodeRunner | undefined }) {
  const block = (
    <pre className="armada-prose__block" data-acts="1">
      <code>{command}</code>
    </pre>
  );
  if (runner !== undefined) {
    return (
      <RunnableBlock command={command} runner={runner} copy={command}>
        {block}
      </RunnableBlock>
    );
  }
  return (
    <div className="armada-prose__run">
      <div className="armada-prose__run-block">
        {block}
        <span className="armada-prose__run-acts">
          <CopyAct text={command} label={label} />
        </span>
      </div>
    </div>
  );
}

export function StepsPanel({ name, address, command, steps, keep, onOpenAddress, onOpenInBrowser, onSend, runner, onFailed }: StepsPanelProps) {
  const [ticked, setTicked] = useKept<number[]>(`${keep}:ticked`, NO_TICKS);
  const [fixes, setFixes] = useKept<StepFixes>(`${keep}:fixes`, NO_FIXES);
  const [text, setText] = useState("");
  // The step being worded: an index into the original steps then the added ones.
  const [editing, setEditing] = useState<{ at: number; draft: string } | undefined>();
  const total = steps.length + fixes.added.length;
  const lineOf = (at: number) => (at < steps.length ? (fixes.edited[at] ?? steps[at]!) : fixes.added[at - steps.length]!);
  // A command a step already names is copied there, so the top does not say it twice.
  const named = command !== undefined && steps.some((step) => partsOf(step).commands.includes(command));

  /** The runner for one command: a clean exit ticks the step it is in, a failed one tells the Job. A stop says nothing. */
  const watched = (command: string, at?: number): CodeRunner | undefined => {
    if (runner === undefined) return undefined;
    return (_run, on, size) => {
      let out = "";
      return runner(
        command,
        (event) => {
          if (event.kind === "out") out += event.text;
          else if (event.code === 0) {
            if (at !== undefined) setTicked((was) => (was.includes(at) ? was : [...was, at]));
          } else if (event.code !== null) {
            const tail = plainOf(out).trim().split("\n").slice(-FAILED_LINES).join("\n");
            const where = at === undefined ? "" : `Step ${at + 1}: `;
            onFailed?.(`${where}\`${command}\` failed, exit code ${event.code}.${tail === "" ? "" : `\n\n${tail}`}`);
          }
          on(event);
        },
        size,
      );
    };
  };

  const send = () => {
    const message = messageOf(text.trim() === "" ? DONE : text, steps, fixes);
    onSend(message);
    setText("");
    forgetKept(`${keep}:reply`);
    forgetKept(`${keep}:ticked`);
    forgetKept(`${keep}:fixes`);
  };
  const save = () => {
    if (editing === undefined) return;
    const { at, draft } = editing;
    const words = draft.trim();
    setEditing(undefined);
    if (at >= steps.length) {
      const place = at - steps.length;
      setFixes((was) => ({ ...was, added: words === "" ? was.added.filter((_, one) => one !== place) : was.added.map((one, here) => (here === place ? words : one)) }));
      return;
    }
    setFixes((was) => {
      const edited = { ...was.edited };
      if (words === "" || words === steps[at]) delete edited[at];
      else edited[at] = words;
      return { ...was, edited };
    });
  };
  const cancel = () => {
    // An added step that was never worded goes with the edit.
    if (editing !== undefined && editing.at >= steps.length && lineOf(editing.at) === "") {
      setFixes((was) => ({ ...was, added: was.added.slice(0, -1) }));
    }
    setEditing(undefined);
  };
  const drop = (at: number) => {
    setFixes((was) => ({ ...was, dropped: [...was.dropped, at] }));
    setTicked((was) => was.filter((one) => one !== at));
  };
  const remove = (at: number) => setFixes((was) => ({ ...was, added: was.added.filter((_, one) => one !== at - steps.length) }));

  return (
    <div className="armada-step-list">
      {address === undefined ? null : (
        <div className="armada-step-list__link">
          <Tooltip label="Opens the page" asChild>
            <Button variant="secondary" size="sm" onClick={() => onOpenAddress(address)}>
              <span className="mono">{address}</span>
            </Button>
          </Tooltip>
          <Tooltip label="Open in browser">
            <Button variant="ghost" size="sm" iconOnly aria-label="Open in browser" onClick={() => onOpenInBrowser(address)}>
              <ExternalLink size={12} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        </div>
      )}
      {command === undefined || named ? null : <CommandToCopy command={command} label="Copy command" runner={watched(command)} />}
      <ol className="armada-guide-steps armada-step-list__list">
        {Array.from({ length: total }, (_, at) => {
          const line = lineOf(at);
          const dropped = fixes.dropped.includes(at);
          const changed = at < steps.length && fixes.edited[at] !== undefined;
          const own = at >= steps.length;
          const parts = partsOf(line);
          return (
            <li
              key={at}
              className="armada-guide-steps__step armada-step-list__step"
              data-ticked={ticked.includes(at) || undefined}
              data-dropped={dropped || undefined}
            >
              <span className="armada-guide-steps__ordinal armada-step-list__ordinal mono">{at + 1}</span>
              {editing?.at === at ? (
                <Input
                  autoFocus
                  aria-label={`Step ${at + 1}`}
                  value={editing.draft}
                  onChange={(event) => setEditing({ at, draft: event.target.value })}
                  onBlur={save}
                  onKeyDown={(event) => {
                    if (event.key === "Escape") {
                      event.preventDefault();
                      event.stopPropagation();
                      cancel();
                    } else if (event.key === "Enter" && !event.nativeEvent.isComposing) {
                      event.preventDefault();
                      save();
                    }
                  }}
                />
              ) : (
                <div className="armada-step-list__body">
                  <Checkbox
                    checked={ticked.includes(at)}
                    disabled={dropped}
                    onChange={() => setTicked((was) => (was.includes(at) ? was.filter((one) => one !== at) : [...was, at]))}
                  >
                    {parts.words}
                  </Checkbox>
                  {dropped ? null : parts.commands.map((command, which) => <CommandToCopy key={which} command={command} label={`Copy step ${at + 1} command`} runner={watched(command, at)} />)}
                </div>
              )}
              {editing?.at === at ? null : (
                <span className="armada-step-list__acts">
                  {!changed ? null : (
                    <Tooltip label={steps[at]!}>
                      <span className="armada-step-list__changed" role="img" aria-label={`Step ${at + 1} was ${steps[at]!}`}>
                        <UserPen size={12} strokeWidth={2} aria-hidden />
                      </span>
                    </Tooltip>
                  )}
                  <span className="armada-step-list__keys">
                    {dropped ? (
                      <Tooltip label="Bring back">
                        <Button
                          variant="ghost"
                          size="sm"
                          iconOnly
                          aria-label={`Bring back step ${at + 1}`}
                          onClick={() => setFixes((was) => ({ ...was, dropped: was.dropped.filter((one) => one !== at) }))}
                        >
                          <Undo2 size={12} strokeWidth={2} aria-hidden />
                        </Button>
                      </Tooltip>
                    ) : (
                      <>
                        <Tooltip label="Edit step">
                          <Button variant="ghost" size="sm" iconOnly aria-label={`Edit step ${at + 1}`} onClick={() => setEditing({ at, draft: line })}>
                            <SquarePen size={12} strokeWidth={2} aria-hidden />
                          </Button>
                        </Tooltip>
                        <Tooltip label="Drop step">
                          <Button variant="ghost" size="sm" iconOnly aria-label={`Drop step ${at + 1}`} onClick={() => (own ? remove(at) : drop(at))}>
                            <Trash2 size={12} strokeWidth={2} aria-hidden />
                          </Button>
                        </Tooltip>
                      </>
                    )}
                  </span>
                </span>
              )}
            </li>
          );
        })}
      </ol>
      <div className="armada-step-list__add">
        <Tooltip label="Add step">
          <Button
            variant="ghost"
            size="sm"
            iconOnly
            aria-label="Add step"
            onClick={() => {
              setFixes((was) => ({ ...was, added: [...was.added, ""] }));
              setEditing({ at: total, draft: "" });
            }}
          >
            <Plus size={12} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      </div>
      <div className="armada-step-list__reply">
        <Textarea
          rows={2}
          keep={`${keep}:reply`}
          aria-label={`Reply to ${name}`}
          value={text}
          onChange={(event) => setText(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
              event.preventDefault();
              if (text.trim() !== "") send();
            }
          }}
        />
        <Button variant="secondary" size="sm" onClick={send}>
          {text.trim() === "" ? DONE : "Send"}
        </Button>
      </div>
    </div>
  );
}
