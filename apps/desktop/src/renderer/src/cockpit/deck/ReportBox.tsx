// The dialog Report opens over the whole window: the owner types what went wrong, and Send or Enter hands it to a new Session. Esc or
// Cancel puts it away and keeps what was typed; a send clears it. Empty words cannot be sent. What starts the Session and what it is
// told is in `SessionTiles.tsx` and `report-brief.ts`. The owner's ask of 15 Oct 2026; drawn over the app rather than in the flight
// controls' place since his walk note of 21 Oct 2026, where the panel clipped it.

import { useState } from "react";
import { Dialog, Textarea, forgetKept } from "@armada/components";

import { OverTheApp } from "../../session-view/OverTheApp";

/** `keep` names the draft, so leaving and coming back finds it. `send` resolves to whether the Session was started and told. */
export function ReportBox({ keep, send, onClose }: { keep: string; send: (words: string) => Promise<boolean>; onClose: () => void }) {
  const [text, setText] = useState("");
  const [sending, setSending] = useState(false);
  const go = async () => {
    const words = text.trim();
    if (words === "" || sending) return;
    setSending(true);
    const done = await send(words);
    setSending(false);
    if (!done) return;
    forgetKept(keep);
    onClose();
  };
  return (
    <OverTheApp open>
      <Dialog open tone="neutral" title="Report an issue with the Job" confirmLabel="Send" confirmDisabled={text.trim() === "" || sending} onConfirm={() => void go()} onCancel={onClose}>
        <Textarea autoFocus rows={3} keep={keep} aria-label="Report" value={text} onChange={(event) => setText(event.target.value)} />
      </Dialog>
    </OverTheApp>
  );
}
