import { THOUGHT_KIND } from "../ThoughtCard/ThoughtCard";
import type { ThoughtTarget } from "../ThoughtCard/ThoughtCard";

/** One compact line under the owner's words: where they went. Pressing it opens that Session or Job. */
export function SentTo({ target, onOpen }: { target: ThoughtTarget; onOpen: () => void }) {
  const { Glyph, said } = THOUGHT_KIND[target.kind];
  return (
    <div className="armada-helm-sent">
      <span className="armada-helm-sent__label">Sent to</span>
      <button type="button" className="armada-helm-sent__chip" aria-label={`Sent to ${said} ${target.title}`} onClick={onOpen}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
        <span className="armada-helm-sent__title">{target.title}</span>
        <span className="armada-helm-sent__repository mono">{target.repository}</span>
      </button>
    </div>
  );
}
