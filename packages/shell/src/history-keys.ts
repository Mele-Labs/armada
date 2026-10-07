// `⌘[` and `⌘]` — back and forward through the places a person has been. The
// binding is the registry's `history` row, a pair in one shortcut, split here.
// The mouse's back and forward buttons, 3 and 4, are the same input.

import { useEffect, useRef } from "react";
import { keyFor } from "@armada/components";

const [BACK, FORWARD] = keyFor("history").split(" ").map((chord) => chord.slice(-1));

/** Whether a press landed where `⌘[` and `⌘]` already mean outdent and indent. */
function inEditor(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && (target.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName));
}

/** Back on `⌘[` or button 3, forward on `⌘]` or button 4; the keys not from a field being typed in. */
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
    function clicked(event: MouseEvent): void {
      if (event.button !== 3 && event.button !== 4) return;
      event.preventDefault();
      if (event.button === 3) latest.current.onBack();
      else latest.current.onForward();
    }
    // `auxclick` follows `mouseup` for these buttons; refusing it keeps the page from its own navigation.
    const refuse = (event: MouseEvent): void => {
      if (event.button === 3 || event.button === 4) event.preventDefault();
    };
    window.addEventListener("keydown", pressed);
    window.addEventListener("mouseup", clicked);
    window.addEventListener("auxclick", refuse);
    return () => {
      window.removeEventListener("keydown", pressed);
      window.removeEventListener("mouseup", clicked);
      window.removeEventListener("auxclick", refuse);
    };
  }, []);
}
