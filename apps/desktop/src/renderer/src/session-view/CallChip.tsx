// What waits on him is the thread's last card, answered where it stands. Scrolled away from it, a chip over the composer names it and jumps back,
// putting focus in the card. Gone while the card is in view.

import { useEffect, useState, type RefObject } from "react";
import { ArrowDown } from "lucide-react";
import type { Session } from "@armada/screens/src/draft/sessions";

/** The word for what the thread's last card holds, the one the agent is held on first: a question, a permission, a plan, a judge's concern, or a note. */
export function callWord(session: Session, notes: number): string | undefined {
  const asked = session.asked;
  if (asked !== undefined) return (asked.questions?.length ?? 0) > 0 ? "Question" : "Permission";
  const plan = session.work?.plan;
  if (plan !== undefined && !plan.approved) return "Plan";
  if ((session.work?.concerns?.length ?? 0) > 0) return "Judge";
  return notes > 0 ? "Needs you" : undefined;
}

export function CallChip({ word, scope }: { word: string | undefined; scope: RefObject<HTMLElement | null> }) {
  const [away, setAway] = useState(false);
  useEffect(() => {
    const scroller = scope.current?.querySelector<HTMLElement>(".armada-session-thread__rows");
    if (scroller === null || scroller === undefined) return;
    const read = () => {
      const tail = scroller.querySelector<HTMLElement>(".armada-session-thread__tail");
      if (tail === null) return setAway(false);
      const view = scroller.getBoundingClientRect();
      const box = tail.getBoundingClientRect();
      setAway(box.top > view.bottom - 24);
    };
    read();
    scroller.addEventListener("scroll", read, { passive: true });
    const watch = new ResizeObserver(read);
    watch.observe(scroller);
    return () => {
      scroller.removeEventListener("scroll", read);
      watch.disconnect();
    };
  }, [scope, word]);
  if (word === undefined || !away) return null;
  const jump = () => {
    const tail = scope.current?.querySelector<HTMLElement>(".armada-session-thread__tail");
    tail?.scrollIntoView({ block: "end", behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth" });
    tail?.querySelector<HTMLElement>("button, textarea, input")?.focus({ preventScroll: true });
  };
  return (
    <div className="sv-callchip">
      <button type="button" className="sv-callchip__press" onClick={jump}>
        <span>{word}</span>
        <ArrowDown size={12} strokeWidth={2} aria-hidden="true" />
      </button>
    </div>
  );
}
