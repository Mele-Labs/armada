// The lens's card at the Field's top right: the file a drone of the Job is writing, its counts, and the newest hunk of its change, the
// lines new since the last read typed out once and then still. The files the Job wrote before it stack behind as faded tabs. Under
// reduced motion the hunk shows whole at once. DOM, so a new answer changes its text and nothing on the canvas.

import { memo, useEffect, useState } from "react";

import { countsOf, type LensLine } from "../cockpit/deck/lens";
import type { CardIn } from "./field-lens";

/** The most lines of the hunk the card holds: its newest. The Field large has room for more. */
const LINES = 6;
const LINES_LARGE = 14;
/** How fast the new lines type out, and the longest a reveal takes however much is new. */
const CHARS_PER_S = 160;
const REVEAL_MS = 1_200;
/** A reveal draws again at most this often. */
const STEP_MS = 33;

const still = (): boolean => typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

const base = (path: string): string => path.split("/").pop() || path;

const MARK: Record<LensLine["kind"], string> = { added: "+", removed: "-", context: " " };

/** How many characters of the new lines are shown, counting up from none to `total` once per `n`. Whole at once under reduced motion. */
function useReveal(total: number, n: number | undefined, onTyping: (typing: boolean) => void): number {
  const [shown, setShown] = useState(total);
  useEffect(() => {
    if (total === 0 || n === undefined || still()) {
      setShown(total);
      return;
    }
    const rate = Math.max(CHARS_PER_S, (total * 1000) / REVEAL_MS);
    const start = performance.now();
    let last = 0;
    let frame = 0;
    setShown(0);
    onTyping(true);
    const step = (now: number) => {
      const at = Math.min(total, Math.floor(((now - start) / 1000) * rate));
      if (at >= total) {
        setShown(total);
        onTyping(false);
        return;
      }
      if (now - last >= STEP_MS) {
        last = now;
        setShown(at);
      }
      frame = requestAnimationFrame(step);
    };
    frame = requestAnimationFrame(step);
    return () => {
      cancelAnimationFrame(frame);
      onTyping(false);
    };
  }, [n]);
  return Math.min(shown, total);
}

export const FieldLensCard = memo(function FieldLensCard({ card, large, onTyping }: { card: CardIn; large: boolean; onTyping: (typing: boolean) => void }) {
  const { diff, newest } = card;
  const most = large ? LINES_LARGE : LINES;
  const all = newest?.hunk.lines ?? [];
  const lines = all.slice(-most);
  const skipped = all.length - lines.length;
  // Only the lines new since the last read type out, and only those the card holds.
  const read = Math.max(0, (newest?.read ?? all.length) - skipped);
  const fresh = lines.slice(read).reduce((sum, one) => sum + one.text.length + 1, 0);
  const shown = useReveal(fresh, newest === undefined || read >= lines.length ? undefined : newest.n, onTyping);
  let left = shown;
  return (
    <div className="sv-lens" role="group" aria-label={`Lens: ${base(card.path)}`} data-large={large || undefined} style={{ ["--tabs" as string]: card.tabs.length }}>
      {card.tabs
        .map((tab, k) => (
          <div key={tab.path} className="sv-lens__tab" data-k={k + 1}>
            <span className="sv-lens__name">{base(tab.path)}</span>
            {tab.diff === undefined ? null : <span className="sv-lens__counts">{countsOf(tab.diff)}</span>}
          </div>
        ))
        .reverse()}
      <div className="sv-lens__card" data-lens-front="">
        <div className="sv-lens__head">
          <span className="sv-lens__name">{base(card.path)}</span>
          <span className="sv-lens__counts">{countsOf(diff)}</span>
        </div>
        {lines.length === 0 ? null : (
          <div className="sv-lens__lines">
            {lines.map((one, k) => {
              const full = `${one.text}\n`;
              let text = one.text;
              let typing = false;
              if (k >= read) {
                const take = Math.max(0, Math.min(full.length, left));
                left -= full.length;
                if (take === 0) return null;
                typing = take < full.length;
                text = one.text.slice(0, take);
              }
              return (
                <div key={k} className="sv-lens__line" data-kind={one.kind}>
                  <span className="sv-lens__mark" aria-hidden="true">{MARK[one.kind]}</span>
                  <span className="sv-lens__text">{text}</span>
                  {typing ? <span className="sv-lens__caret" aria-hidden="true" /> : null}
                </div>
              );
            })}
          </div>
        )}
        {diff.cut === undefined ? null : <div className="sv-lens__cut">{diff.cut}</div>}
      </div>
    </div>
  );
});
