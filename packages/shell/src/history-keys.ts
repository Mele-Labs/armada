// `⌘[` and `⌘]` — back and forward through the places a person has been. The
// binding is the registry's `history` row, a pair in one shortcut, split here.

import { useEffect, useRef } from "react";
import { keyFor } from "@armada/components";

const [BACK, FORWARD] = keyFor("history").split(" ").map((chord) => chord.slice(-1));

/** Whether a press landed where `⌘[` and `⌘]` already mean outdent and indent. */
function inEditor(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && (target.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName));
}

/** Back on `⌘[`, forward on `⌘]`, from every surface but a field being typed in. */
export function useHistoryKeys(onBack: () => void, onForward: () => void): void {
  const latest = useRef({ onBack, onForward });
  latest.current = { onBack, onForward };

  useEffect(() => {
    function pressed(event: KeyboardEvent): void {
      if (!event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
      if (event.key !== BACK && event.key !== FORWARD) return;
      if (inEditor(event.target)) return;
      event.preventDefault();
      if (event.key === BACK) latest.current.onBack();
      else latest.current.onForward();
    }
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  }, []);
}
