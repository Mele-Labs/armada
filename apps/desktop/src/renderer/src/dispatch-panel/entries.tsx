// The way into the dispatch panel: the one sheet over the one draft (`dispatching.tsx`), centred over the
// window, and the strip a send leaves over the title bar's field.

import { useEffect, useRef, type KeyboardEvent } from "react";

import { ONE_BAR } from "../one-bar";
import { DispatchPanel, useHung } from "./DispatchPanel";
import { DispatchedStrip } from "./DispatchedStrip";
import { useDispatching } from "./dispatching";

/** The sheet, centred over the window, wherever an entry opened it. */
export function DispatchSheet() {
  const dispatching = useDispatching();
  const field = useRef<HTMLTextAreaElement>(null);
  const shown = dispatching?.sheet === true;

  // The cursor lands at the end of the words the palette sent, so typing carries on from them.
  useEffect(() => {
    if (!shown) return;
    const end = field.current?.value.length ?? 0;
    field.current?.focus();
    field.current?.setSelectionRange(end, end);
  }, [shown]);

  if (dispatching === undefined || !shown) return null;
  return <DispatchPanel fieldRef={field} onDismiss={dispatching.closeSheet} {...dispatching.panel()} />;
}

/**
 * What a send from the sheet leaves, laid over the one bar's field.
 * The cursor lands on it, so ↵ opens what was sent, Esc dismisses it, and a key typed starts the next draft.
 */
export function TitleDispatchedStrip({ anchor = ONE_BAR }: { anchor?: string }) {
  const dispatching = useDispatching();
  const root = useRef<HTMLDivElement>(null);
  const sent = dispatching?.sent;
  const shown = sent !== undefined && dispatching?.sheet === false;
  const place = useHung(shown, anchor);

  useEffect(() => {
    if (shown) root.current?.focus();
  }, [shown, sent?.id]);

  if (dispatching === undefined || !shown || sent === undefined) return null;

  const keys = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      dispatching.dismissSent();
    } else if (event.key === "Enter" && event.target === event.currentTarget) {
      event.preventDefault();
      dispatching.openSent();
    } else if (event.key.length === 1 && event.key !== " " && !event.metaKey && !event.ctrlKey && !event.altKey) {
      event.preventDefault();
      dispatching.openSheet(event.key);
    }
  };

  return (
    <DispatchedStrip
      sent={sent}
      rootRef={root}
      {...(place === undefined ? {} : { style: place })}
      onOpen={dispatching.openSent}
      onDismiss={dispatching.dismissSent}
      onKeyDown={keys}
    />
  );
}
