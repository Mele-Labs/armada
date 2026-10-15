// The box Report opens in place of the flight controls' tiles: the owner types what went wrong, and a press of Send or Enter hands it to a
// new Session. Esc puts it away and keeps what was typed; a send clears it. Empty words cannot be sent. What starts the Session and what
// it is told is in `SessionTiles.tsx` and `report-brief.ts`. It stands where the Session's tiles do (`data-subject`), so a digit pressed
// outside the box presses none of the six usual tiles. The owner's ask of 15 Oct 2026.

import { useState } from "react";
import { Button, Textarea, forgetKept } from "@armada/components";

/** `keep` names the draft, so leaving the panel and coming back finds it. `send` resolves to whether the Session was started and told. */
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
    <div className="deck-controls__block deck-report" data-subject="session">
      <Textarea
        autoFocus
        rows={2}
        keep={keep}
        aria-label="Report"
        value={text}
        onChange={(event) => setText(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            onClose();
          } else if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
            event.preventDefault();
            void go();
          }
        }}
      />
      <Button variant="secondary" size="sm" pending={sending} disabled={text.trim() === ""} onClick={() => void go()}>
        Send
      </Button>
    </div>
  );
}
