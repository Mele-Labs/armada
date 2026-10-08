import { useId, useLayoutEffect, useRef } from "react";
import type { TextareaHTMLAttributes } from "react";

import { forgetKept, readKept, writeKept } from "../../keep";

/**
 * A multi-line text field, with its label and its invalid message.
 *
 * `textarea` is sanctioned by hard rule two because a Job's brief is prose a
 * person writes at length, and a single-line input for it is a control that
 * fights its content. The contract specifies no `Textarea` of its own, so
 * surface, edge, radius, type, focus, invalid and disabled all come from
 * `### Input` — the two line up in a column of fields, and a control that
 * behaves differently from an input for no reason is two controls to learn.
 *
 * Three things `Input` does not answer, decided here:
 *
 * - **Height is a row count, not a height.** `rows` sets it, defaulting to
 *   three. The mockup draws an 88px well and no token names that number; a row
 *   count is the same well spelled in the type scale, which is where a
 *   text field's height comes from anyway.
 * - **It does not resize.** A drag handle is chrome, and this is an instrument
 *   panel. A caller that needs a taller well passes `rows`.
 * - **It does not count characters.** No brief length limit is stated
 *   anywhere, and a counter would be the first place one was invented.
 *
 * **A text box someone types into keeps what was typed.** `keep` names what
 * the text is about (`helm:armada`, `session:01J…`); leave the panel and come
 * back, or reload Bridge, and the text is there with the caret at its end. The
 * text clears when the field's value goes empty, which is what a send does.
 * `keep={false}` is for a field that edits a stored value, whose text lives
 * there already. It defaults to `false` until every call site names its key.
 *
 * Field labels never open with a Wh- word. The label is `Brief`.
 */
export type TextareaProps = TextareaHTMLAttributes<HTMLTextAreaElement> & {
  /** Sentence case, no Wh- opener. Omitted where a surface labels the field itself. */
  label?: string;
  /** The border goes to `--status-completed-failed` and `message` renders below. */
  invalid?: boolean;
  /** What is wrong, and what to do about it. Rendered only when `invalid`. */
  message?: string;
  /**
   * Grows with its text, from `rows` up to half the height of the container
   * its form sits in, and scrolls past that. Shrinks as text is removed.
   */
  grow?: boolean;
  /** What the text is about, or `false` for a field that edits a stored value. */
  keep?: string | false;
};

export function Textarea({
  label,
  invalid = false,
  message,
  grow = false,
  keep = false,
  rows = 3,
  id,
  ...rest
}: TextareaProps) {
  const generated = useId();
  const textareaId = id ?? generated;
  const messageId = `${textareaId}-message`;
  const showMessage = invalid && message !== undefined;
  const field = useRef<HTMLTextAreaElement>(null);

  // Mount: put back what was kept, through the field's own input event so a controlled host hears it.
  useLayoutEffect(() => {
    const box = field.current;
    if (keep === false || box === null || box.value !== "") return;
    const kept = readKept<string>(keep);
    if (kept === undefined || kept === "") return;
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set?.call(box, kept);
    box.dispatchEvent(new Event("input", { bubbles: true }));
    box.setSelectionRange(kept.length, kept.length);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [keep]);

  const firstRun = useRef(true);
  const remember = (text: string) => {
    if (keep === false) return;
    if (text === "") forgetKept(keep);
    else writeKept(keep, text);
  };
  useLayoutEffect(() => {
    if (firstRun.current) {
      firstRun.current = false;
      return;
    }
    if (rest.value !== undefined) remember(String(rest.value));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rest.value]);

  useLayoutEffect(() => {
    const box = field.current;
    if (!grow || box === null) return;
    const bound = box.closest("form")?.parentElement;
    box.style.height = "auto";
    const natural = box.scrollHeight + box.offsetHeight - box.clientHeight;
    const cap = bound ? bound.clientHeight / 2 : Infinity;
    box.style.height = `${Math.max(Math.min(natural, cap), box.offsetHeight)}px`;
  }, [grow, rest.value]);

  return (
    <div className="armada-textarea-field">
      {label !== undefined && (
        <label className="armada-textarea-field__label" htmlFor={textareaId}>
          {label}
        </label>
      )}
      <textarea
        {...rest}
        onInput={(event) => {
          // An uncontrolled field has no value prop to watch, so its keeping is here.
          if (rest.value === undefined) remember(event.currentTarget.value);
          rest.onInput?.(event);
        }}
        ref={field}
        rows={rows}
        id={textareaId}
        className="armada-textarea"
        aria-invalid={invalid || undefined}
        aria-describedby={showMessage ? messageId : undefined}
      />
      {showMessage && (
        <span className="armada-textarea-field__message" id={messageId}>
          {message}
        </span>
      )}
    </div>
  );
}
