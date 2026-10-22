// The map's hover cards: the one the pointer is on, else the Session he is looking at if it is building something. Held apart from the
// battlefield so that a card coming and going draws the card alone (`tip-hover.ts`). The pointer on a card holds it; `float` draws the
// cards over the app.

import { useSyncExternalStore } from "react";

import { Callout, type Anchor } from "./Callout";
import type { Tip } from "./tip";
import type { Tips } from "./tip-hover";

export function TipLayer({ tips, host, view, locked, float = false }: { tips: Tips; host: HTMLElement; view: unknown; locked?: { tip: Tip; anchor: Anchor; key: string | undefined }; float?: boolean }) {
  const shown = useSyncExternalStore(tips.subscribe, tips.get);
  if (shown !== undefined) return <Callout tip={shown.tip} anchor={shown.anchor} host={host} view={view} float={float} onEnter={tips.hold} onLeave={tips.leave} />;
  // The Session's own waits for the pointer to leave.
  if (locked === undefined) return null;
  return <Callout key={`locked:${locked.key}`} tip={locked.tip} anchor={locked.anchor} host={host} view={view} float={float} pinned />;
}
