import { useState } from "react";

import { forgetKept, useKept } from "../../keep";
import { Button } from "../../primitives/Button/Button";
import { Checkbox } from "../../primitives/Checkbox/Checkbox";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * What a waiting item opens when it is a procedure: the link as one key at the top, the steps numbered
 * as a guide's are with a tick box each, and under them the box the answer is typed in. **Ticks and the
 * reply are kept under `keep`**, so closing the panel or leaving the Session loses neither; a send
 * clears both, the item being answered.
 */
export type StepsPanelProps = {
  /** The item's words, which name the reply box. */
  name: string;
  /** The page the steps are about, drawn on the key at the top that opens it. */
  address?: string;
  steps: readonly string[];
  /** What the ticks and the draft are about: the Session and the item. */
  keep: string;
  onOpenAddress: (address: string) => void;
  onSend: (text: string) => void;
};

const NO_TICKS: number[] = [];

export function StepsPanel({ name, address, steps, keep, onOpenAddress, onSend }: StepsPanelProps) {
  const [ticked, setTicked] = useKept<number[]>(`${keep}:ticked`, NO_TICKS);
  const [text, setText] = useState("");
  const send = () => {
    const said = text.trim();
    if (said === "") return;
    onSend(said);
    setText("");
    forgetKept(`${keep}:reply`);
    forgetKept(`${keep}:ticked`);
  };
  return (
    <div className="armada-steps">
      {address === undefined ? null : (
        <div className="armada-steps__link">
          <Tooltip label="Opens the page" asChild>
            <Button variant="secondary" size="sm" onClick={() => onOpenAddress(address)}>
              <span className="mono">{address}</span>
            </Button>
          </Tooltip>
        </div>
      )}
      <ol className="armada-guide-steps armada-steps__list">
        {steps.map((line, index) => (
          <li key={line} className="armada-guide-steps__step armada-steps__step" data-ticked={ticked.includes(index) || undefined}>
            <span className="armada-guide-steps__ordinal armada-steps__ordinal mono">{index + 1}</span>
            <Checkbox
              checked={ticked.includes(index)}
              onChange={() => setTicked((was) => (was.includes(index) ? was.filter((one) => one !== index) : [...was, index]))}
            >
              {line}
            </Checkbox>
          </li>
        ))}
      </ol>
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
        <Button variant="secondary" size="sm" disabled={text.trim() === ""} onClick={send}>
          Send
        </Button>
      </div>
    </div>
  );
}
