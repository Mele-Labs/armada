// The two ways into the dispatch panel. Each draws the one panel over the one draft
// (`dispatching.tsx`) and puts it in its own place.

import { useEffect, useRef, type KeyboardEvent } from "react";

import { DispatchPanel, useHung } from "./DispatchPanel";
import { DispatchedStrip } from "./DispatchedStrip";
import { useDispatching } from "./dispatching";

/** The Cockpit's dispatch bar: the line at rest, and typing grows it into the panel where it stands. */
export function CockpitDispatchBar() {
  const dispatching = useDispatching();
  if (dispatching === undefined) return null;
  return <DispatchPanel layout="docked" away={dispatching.titled} {...dispatching.panel("bar")} />;
}

/** The panel grown down from the title bar's field when the one bar chose to dispatch. */
export function TitleDispatchPanel({ anchor }: { anchor: string }) {
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
  return <DispatchPanel layout="anchored" anchor={anchor} fieldRef={field} onDismiss={dispatching.closeTitled} {...dispatching.panel("title")} />;
}

/**
 * What a send from the title bar's panel leaves, laid over the one bar's field where the panel grew from.
 * The cursor lands on it, so ↵ opens what was sent, Esc dismisses it, and a key typed starts the next draft.
 */
export function TitleDispatchedStrip({ anchor }: { anchor: string }) {
  const dispatching = useDispatching();
  const root = useRef<HTMLDivElement>(null);
  const sent = dispatching?.sent;
  const shown = sent !== undefined && sent.entry === "title" && dispatching?.titled === false;
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
      layout="anchored"
      rootRef={root}
      {...(place === undefined ? {} : { style: place })}
      onOpen={dispatching.openSent}
      onDismiss={dispatching.dismissSent}
      onKeyDown={keys}
    />
  );
}
