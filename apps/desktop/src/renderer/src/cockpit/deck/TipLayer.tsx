// The map's hover cards: the one the pointer is on, else the Session he is looking at if it is building something. Held apart from the
// battlefield so that a card coming and going draws the card alone (`tip-hover.ts`).

import { useSyncExternalStore } from "react";

import { Callout, type Anchor } from "./Callout";
import type { Tip } from "./tip";
import type { Tips } from "./tip-hover";

export function TipLayer({ tips, host, view, held, locked }: { tips: Tips; host: HTMLElement; view: unknown; held: boolean; locked?: { tip: Tip; anchor: Anchor; key: string | undefined } }) {
  const shown = useSyncExternalStore(tips.subscribe, tips.get);
  if (shown !== undefined) return <Callout tip={shown.tip} anchor={shown.anchor} host={host} view={view} />;
  // A card pressed open is the one that stays; the Session's own waits for the pointer to leave.
  if (held || locked === undefined) return null;
  return <Callout key={`locked:${locked.key}`} tip={locked.tip} anchor={locked.anchor} host={host} view={view} pinned />;
}
