import { createContext, useEffect, useState } from "react";
import { Check, Play } from "lucide-react";

import { Tooltip } from "../Tooltip/Tooltip";

/**
 * Sends a Claude Code slash command to the Session's terminal, to run as typed. Bridge provides one
 * for a terminal Session only; absent, inline code is drawn as plain code.
 */
export const ProseCommand = createContext<((command: string) => void) | null>(null);

/** Inline code that is exactly a slash command: `/update-armada`, `/model opus`. */
export const SLASH_COMMAND = /^\/[a-z][a-z0-9:-]*( .*)?$/;

/** How long the sent mark stays before Run is offered again. */
const SENT_MS = 2400;

/** Inline code with a quiet Run on hover or focus. A press sends it, and a check says so. */
export function SlashCommand({ text, send }: { text: string; send: (command: string) => void }) {
  const [sent, setSent] = useState(false);
  useEffect(() => {
    if (!sent) return;
    const handle = window.setTimeout(() => setSent(false), SENT_MS);
    return () => window.clearTimeout(handle);
  }, [sent]);
  return (
    <Tooltip label={sent ? "Sent" : "Run in this Session"} asChild>
      <button
        type="button"
        className="armada-prose__code armada-prose__command"
        data-sent={sent ? "" : undefined}
        onClick={() => {
          send(text);
          setSent(true);
        }}
      >
        {text}
        {sent ? (
          <span className="armada-prose__command-mark" role="img" aria-label="Sent">
            <Check size={10} strokeWidth={2} aria-hidden />
          </span>
        ) : (
          <Play size={10} strokeWidth={2} aria-hidden className="armada-prose__command-play" />
        )}
      </button>
    </Tooltip>
  );
}
