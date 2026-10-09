// The two ways into the dispatch panel. Each draws the one panel over the one draft
// (`dispatching.tsx`) and puts it in its own place.

import { useEffect, useRef } from "react";

import { DispatchPanel } from "./DispatchPanel";
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
