import { useState } from "react";
import { ExternalLink, Pencil, PencilLine, Plus, Trash2, Undo2 } from "lucide-react";

import { forgetKept, useKept } from "../../keep";
import { Button } from "../../primitives/Button/Button";
import { Checkbox } from "../../primitives/Checkbox/Checkbox";
import { Input } from "../../primitives/Input/Input";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * What a waiting item opens when it is a procedure: the link as one key at the top, the steps numbered
 * as a guide's are with a tick box each, and under them the box the answer is typed in.
 *
 * **The agent may be wrong about the steps, and the person says so here.** A step is edited in place,
 * dropped (it stays in the list, struck through, so the agent sees what was wrong) or added at the end.
 * Ticks are the person's own place-keeping and nothing depends on them. **Corrections travel with Send**,
 * as plain lines after the reply (`messageOf`). Ticks, corrections and the reply are kept under `keep`,
 * so closing the panel or leaving the Session loses none; a send clears all three.
 */
export type StepsPanelProps = {
  /** The item's words, which name the reply box. */
  name: string;
  /** The page the steps are about, drawn on the key at the top that opens it. */
  address?: string;
  steps: readonly string[];
  /** What the ticks, the corrections and the draft are about: the Session and the item. */
  keep: string;
  /** Opens the page in an Armada window. */
  onOpenAddress: (address: string) => void;
  /** Opens the page in the person's own browser. */
  onOpenInBrowser: (address: string) => void;
  onSend: (text: string) => void;
};

/** What the person changed: new words by step index, steps dropped, and steps added after the last. */
export type StepFixes = { edited: Record<number, string>; dropped: number[]; added: string[] };

const NO_TICKS: number[] = [];
const NO_FIXES: StepFixes = { edited: {}, dropped: [], added: [] };

const fixed = (fixes: StepFixes) => Object.keys(fixes.edited).length > 0 || fixes.dropped.length > 0 || fixes.added.length > 0;

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

export function StepsPanel({ name, address, steps, keep, onOpenAddress, onOpenInBrowser, onSend }: StepsPanelProps) {
  const [ticked, setTicked] = useKept<number[]>(`${keep}:ticked`, NO_TICKS);
  const [fixes, setFixes] = useKept<StepFixes>(`${keep}:fixes`, NO_FIXES);
  const [text, setText] = useState("");
  // The step being worded: an index into the original steps then the added ones.
  const [editing, setEditing] = useState<{ at: number; draft: string } | undefined>();
  const total = steps.length + fixes.added.length;
  const lineOf = (at: number) => (at < steps.length ? (fixes.edited[at] ?? steps[at]!) : fixes.added[at - steps.length]!);

  const send = () => {
    const message = messageOf(text, steps, fixes);
    if (message === "") return;
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
    <div className="armada-steps">
      {address === undefined ? null : (
        <div className="armada-steps__link">
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
      <ol className="armada-guide-steps armada-steps__list">
        {Array.from({ length: total }, (_, at) => {
          const line = lineOf(at);
          const dropped = fixes.dropped.includes(at);
          const changed = at < steps.length && fixes.edited[at] !== undefined;
          const own = at >= steps.length;
          return (
            <li
              key={at}
              className="armada-guide-steps__step armada-steps__step"
              data-ticked={ticked.includes(at) || undefined}
              data-dropped={dropped || undefined}
            >
              <span className="armada-guide-steps__ordinal armada-steps__ordinal mono">{at + 1}</span>
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
                <Checkbox
                  checked={ticked.includes(at)}
                  disabled={dropped}
                  onChange={() => setTicked((was) => (was.includes(at) ? was.filter((one) => one !== at) : [...was, at]))}
                >
                  {line}
                </Checkbox>
              )}
              {editing?.at === at ? null : (
                <span className="armada-steps__acts">
                  {!changed ? null : (
                    <Tooltip label={steps[at]!}>
                      <span className="armada-steps__changed" role="img" aria-label={`Step ${at + 1} was ${steps[at]!}`}>
                        <PencilLine size={12} strokeWidth={2} aria-hidden />
                      </span>
                    </Tooltip>
                  )}
                  <span className="armada-steps__keys">
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
                            <Pencil size={12} strokeWidth={2} aria-hidden />
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
      <div className="armada-steps__add">
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
      <div className="armada-steps__reply">
        <Textarea
          rows={2}
          keep={`${keep}:reply`}
          aria-label={`Reply to ${name}`}
          value={text}
          onChange={(event) => setText(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
              event.preventDefault();
              send();
            }
          }}
        />
        <Button variant="secondary" size="sm" disabled={text.trim() === "" && !fixed(fixes)} onClick={send}>
          Send
        </Button>
      </div>
    </div>
  );
}
