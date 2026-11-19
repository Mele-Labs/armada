import { SquareTerminal } from "lucide-react";

/** Where words went: the Job's title, over its repository. */
export type SentTarget = {
  kind: "session";
  title: string;
  /** The repository's name, as the picker draws it. */
  repository: string;
  /** A Session that holds a slot: hovering its name draws the Session's card. */
  slot?: number;
};

/** The kind's glyph and the word for it, which are the Session card's own. */
const SENT_KIND = {
  session: {
    Glyph: SquareTerminal,
    said: "Job",
  },
} as const;

/** One compact line under the owner's words: where they went. Pressing it opens that Session. */
export function SentTo({ target, onOpen }: { target: SentTarget; onOpen: () => void }) {
  const { Glyph, said } = SENT_KIND[target.kind];
  return (
    <div className="armada-sent-to">
      <span className="armada-sent-to__label">Sent to</span>
      <button
        type="button"
        className="armada-sent-to__chip"
        aria-label={`Sent to ${said} ${target.title}`}
        onClick={onOpen}
      >
        <Glyph size={12} strokeWidth={2} aria-hidden />
        <span className="armada-sent-to__title">{target.title}</span>
        <span className="armada-sent-to__repository mono">{target.repository}</span>
      </button>
    </div>
  );
}
