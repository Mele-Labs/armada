import { createContext, useEffect, useState } from "react";
import { Check, Play } from "lucide-react";

import { Tooltip } from "../Tooltip/Tooltip";

/**
 * What a terminal Session's thread can send: the command to its terminal, to run as typed, and the
 * names it lists (its skills and commands, the built-ins among them). Bridge provides it for a terminal
 * Session only; absent, inline code is drawn as plain code.
 */
export type SlashCommands = { send: (command: string) => void; names: ReadonlySet<string> };

export const ProseCommand = createContext<SlashCommands | null>(null);

const SLASH_COMMAND = /^\/([a-z][a-z0-9:-]*)( .*)?$/;

/** Whether inline code is exactly a slash command the Session lists: `/update-armada`, `/model opus`, and not `/tmp`. */
export function isSlashCommand(text: string, names: ReadonlySet<string>): boolean {
  const name = SLASH_COMMAND.exec(text)?.[1];
  return name !== undefined && names.has(name);
}

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
    <Tooltip label={sent ? "Sent" : "Run in this Job"} asChild>
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
