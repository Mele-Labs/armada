// The fields the Workflows inspector draws its rows with. Each takes its name from its row's label (`labelledBy`), or a name of its own where a row holds several.

import { useEffect, useId, useState } from "react";

import { Input } from "../../primitives/Input/Input";
import { Select } from "../../primitives/Select/Select";
import { Textarea } from "../../primitives/Textarea/Textarea";

/** A field that edits a stored value and says it where it is left, or on Enter. Its name is its row's label: `labelledBy` is that label's id. */
export function Field({ labelledBy, label, value, onCommit, mono = false }: { labelledBy?: string; label?: string; value: string; onCommit: (next: string) => void; mono?: boolean }) {
  const [text, setText] = useState(value);
  useEffect(() => setText(value), [value]);
  const commit = () => {
    const next = text.trim();
    if (next !== value) onCommit(next);
  };
  return (
    <Input
      {...(labelledBy === undefined ? {} : { "aria-labelledby": labelledBy })}
      {...(label === undefined ? {} : { "aria-label": label, placeholder: label })}
      mono={mono}
      value={text}
      onChange={(event) => setText(event.target.value)}
      onBlur={commit}
      onKeyDown={(event) => {
        if (event.key === "Enter") commit();
      }}
    />
  );
}

/** Words that run to several lines, shown whole: the box grows with them. Said where it is left, or on Enter; Shift and Enter starts a line. */
export function Wrapped({ labelledBy, label, keep, value, onCommit }: { labelledBy?: string; label?: string; keep: string; value: string; onCommit: (next: string) => void }) {
  const [text, setText] = useState(value);
  useEffect(() => setText(value), [value]);
  const commit = () => {
    const next = text.trim();
    if (next !== value) onCommit(next);
  };
  return (
    <Textarea
      {...(labelledBy === undefined ? {} : { "aria-labelledby": labelledBy })}
      {...(label === undefined ? {} : { "aria-label": label, placeholder: label })}
      grow
      rows={1}
      keep={keep}
      value={text}
      onChange={(event) => setText(event.target.value)}
      onBlur={commit}
      onKeyDown={(event) => {
        if (event.key !== "Enter" || event.shiftKey) return;
        event.preventDefault();
        commit();
      }}
    />
  );
}

/** Lines of text that edit a stored list, said where the box is left. `keep` names what the lines are about, so a line half typed is there when the panel comes back. */
export function Lines({ labelledBy, label, keep, lines, onCommit }: { labelledBy?: string; label?: string; keep: string; lines: readonly string[]; onCommit: (next: string[]) => void }) {
  const [text, setText] = useState(lines.join("\n"));
  useEffect(() => setText(lines.join("\n")), [lines]);
  return (
    <Textarea
      {...(labelledBy === undefined ? {} : { "aria-labelledby": labelledBy })}
      {...(label === undefined ? {} : { "aria-label": label, placeholder: label })}
      rows={Math.max(2, lines.length)}
      keep={keep}
      value={text}
      onChange={(event) => setText(event.target.value)}
      onBlur={() => {
        const next = text.split("\n").map((one) => one.trim()).filter((one) => one !== "");
        if (next.join("\n") !== lines.join("\n")) onCommit(next);
      }}
    />
  );
}

export type Choice = { value: string; label: string };

/**
 * Two dropdowns on one row that set one fact, a thing and then what it does or is: `[ Build ] [ ends ]`. Both are named by the row's label, the second with a word that tells it from the first, so
 * the row says its name once. Each saves as it is changed. A value the lists do not hold opens on an empty choice.
 */
export function Pair({
  labelledBy,
  first,
  second,
}: {
  labelledBy: string;
  first: { value: string; groups: readonly { label?: string; items: readonly Choice[] }[]; onChange: (value: string) => void };
  second: { value: string; hidden: string; items: readonly Choice[]; onChange: (value: string) => void };
}) {
  const hiddenId = useId();
  const empty = (value: string, items: readonly Choice[]) => value === "" || !items.some((one) => one.value === value);
  const all = first.groups.flatMap((group) => group.items);
  return (
    <div className="armada-wm-pair">
      <span id={hiddenId} className="armada-wm-hidden">
        {second.hidden}
      </span>
      <Select aria-labelledby={labelledBy} value={empty(first.value, all) ? "" : first.value} onChange={(event) => first.onChange(event.target.value)}>
        {empty(first.value, all) ? <option value="" /> : null}
        {first.groups.map((group) =>
          group.label === undefined ? (
            group.items.map((one) => (
              <option key={one.value} value={one.value}>
                {one.label}
              </option>
            ))
          ) : (
            <optgroup key={group.label} label={group.label}>
              {group.items.map((one) => (
                <option key={one.value} value={one.value}>
                  {one.label}
                </option>
              ))}
            </optgroup>
          ),
        )}
      </Select>
      <Select aria-labelledby={`${labelledBy} ${hiddenId}`} value={empty(second.value, second.items) ? "" : second.value} disabled={second.items.length === 0} onChange={(event) => second.onChange(event.target.value)}>
        {empty(second.value, second.items) ? <option value="" /> : null}
        {second.items.map((one) => (
          <option key={one.value} value={one.value}>
            {one.label}
          </option>
        ))}
      </Select>
    </div>
  );
}

