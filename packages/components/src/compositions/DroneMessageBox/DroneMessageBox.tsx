import type { FormEvent, KeyboardEvent } from "react";
import { Button } from "../../primitives/Button/Button";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { SendKbd, sendsOn } from "../../send-message";
import { YourMessage } from "../SessionThread/SessionThread";

/**
 * The message box fixed under an activity log — #1154. Sending it is a
 * redirect, same as the step header's own button and dialog; this is the
 * control for the moment a person is already reading the log and does not
 * want to leave it to reach one.
 *
 * **One component, every mount.** Each caller puts this at its own foot, so
 * no two draw the availability rule two different ways.
 *
 * **Disabled is not blank.** A field that goes grey with nothing said reads as
 * broken; `disabledReason` is the one line a caller gives for why a step with
 * no drone on it cannot take a message.
 */
export type DroneMessageBoxProps = {
  value: string;
  onChange: (value: string) => void;
  onSend: () => void;
  /** No drone is on this step to send to. */
  disabled?: boolean;
  /** Said in the field's place while `disabled` — why, in one line. */
  disabledReason?: string;
  /**
   * The redirect just sent, drawn above the field as the message it is: the
   * row a Session's thread draws for the person's own words, with the same
   * mark while Fleet has not taken it (`sending`) and the same Retry where it
   * did not go (`failed`). **Absent `delivery` is delivered**: the row stays,
   * at rest. The field stays open: sending again is legal and replaces what
   * is outstanding, as it does today.
   */
  sent?: { id: string; at: string; text: string; delivery?: "sending" | "failed" };
  /** Sends the redirect that did not go again. */
  onRetry?: () => void;
  /**
   * A redirect Fleet reports outstanding that this window did not send, so
   * there are no words to draw: `steering.ts`'s own sentence. Shown above the
   * field, and never beside `sent`, which says the same and more.
   */
  waiting?: string;
  placeholder?: string;
};

export function DroneMessageBox({
  value,
  onChange,
  onSend,
  disabled = false,
  disabledReason,
  sent,
  onRetry,
  waiting,
  placeholder = "Message the drone",
}: DroneMessageBoxProps) {
  const blank = value.trim() === "";
  const available = !blank && !disabled;

  function submit(event: FormEvent): void {
    event.preventDefault();
    if (available) onSend();
  }

  // ⌘Enter sends exactly what Send sends, down to sending nothing from a blank
  // or disabled box. Plain Enter falls through to the field as a new line.
  function keyed(event: KeyboardEvent<HTMLTextAreaElement>): void {
    if (!sendsOn(event)) return;
    event.preventDefault();
    if (available) onSend();
  }

  return (
    <form className="armada-drone-message-box" onSubmit={submit}>
      {sent === undefined ? null : (
        <ol className="armada-helm-thread__rows">
          <YourMessage row={{ id: sent.id, at: sent.at, text: sent.text, ...(sent.delivery === undefined ? {} : { delivery: sent.delivery }) }} {...(onRetry === undefined ? {} : { onRetry: () => onRetry() })} />
        </ol>
      )}
      {disabled
        ? disabledReason === undefined
          ? null
          : (
            <p className="armada-drone-message-box__said" role="note">
              {disabledReason}
            </p>
          )
        : waiting === undefined || sent !== undefined
          ? null
          : (
            <p className="armada-drone-message-box__said" role="status">
              {waiting}
            </p>
          )}
      {/* Send sits inside the field's own frame, at its trailing foot, over
          the well rather than beside it — the owner's note of 17 Sep 2026.
          The room it occupies is reserved as the field's bottom padding
          (DroneMessageBox.css), so a typed line never runs underneath it. */}
      <div className="armada-drone-message-box__field">
        <Textarea
          aria-label="Message the drone"
          // Three rows, because a note to a drone is a sentence or two and one
          // row made every one of them scroll while being written.
          rows={3}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          onKeyDown={keyed}
          disabled={disabled}
          placeholder={placeholder}
        />
        {/* `sunken`: Send now rests on the field's own --bg-sunken well, and a
            secondary is filled one surface step from its ground (Button) — on
            `card` it would be the same fill as the field it sits in. */}
        <Button
          type="submit"
          variant="secondary"
          ground="sunken"
          size="sm"
          disabled={disabled || blank}
        >
          Send
          <SendKbd available={available} />
        </Button>
      </div>
    </form>
  );
}
