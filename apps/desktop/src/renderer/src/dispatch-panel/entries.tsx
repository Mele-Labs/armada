// The way into the dispatch panel: the one panel over the one draft (`dispatching.tsx`), grown down
// from the title bar's field, and the strip a send leaves over that field.

import { useEffect, useRef, type KeyboardEvent } from "react";

import { ONE_BAR } from "../one-bar";
import { DispatchPanel, useHung } from "./DispatchPanel";
import { DispatchedStrip } from "./DispatchedStrip";
import { useDispatching } from "./dispatching";
import { placementOfWindow } from "./take";

/** The panel, hung from the title bar's field or centred over the window, wherever an entry opened it. */
export function TitleDispatchPanel() {
  const dispatching = useDispatching();
  const field = useRef<HTMLTextAreaElement>(null);
  const shown = dispatching?.titled === true;

  // The cursor lands at the end of the words the one bar sent, so typing carries on from them.
  useEffect(() => {
    if (!shown) return;
    const end = field.current?.value.length ?? 0;
    field.current?.focus();
    field.current?.setSelectionRange(end, end);
  }, [shown]);

  if (dispatching === undefined || !shown) return null;
  return <DispatchPanel anchor={ONE_BAR} placement={placementOfWindow()} fieldRef={field} onDismiss={dispatching.closeTitled} {...dispatching.panel()} />;
}

/**
 * What a send from the title bar's panel leaves, laid over the one bar's field where the panel grew from.
 * The cursor lands on it, so ↵ opens what was sent, Esc dismisses it, and a key typed starts the next draft.
 */
export function TitleDispatchedStrip({ anchor = ONE_BAR }: { anchor?: string }) {
  const dispatching = useDispatching();
  const root = useRef<HTMLDivElement>(null);
  const sent = dispatching?.sent;
  const shown = sent !== undefined && dispatching?.titled === false;
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
      dispatching.openTitled(event.key);
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
