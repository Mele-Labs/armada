// The Cockpit tab's one body: the flight deck. `cockpit/deck/` holds the parts.

import { useMemo, useRef } from "react";
import { useNow } from "@armada/components";

import { DeckBody } from "./cockpit/deck/Deck";
import { AGE_MS } from "./cockpit/deck/clock";
import type { CockpitProps } from "./cockpit/deck/props";

type Data = "state" | "now" | "picked" | "bring" | "bayActs";
type Act = Exclude<keyof CockpitProps, Data>;

/** Every callback the deck is handed. A record, so a callback added to its props and left off this list does not compile. */
const ACTS: Record<Act, true> = {
  onOpenSession: true,
  onOpenLink: true,
  onOpenCheck: true,
  onOpenLoadout: true,
  onSendToSession: true,
  onTell: true,
  onReleaseSlot: true,
  onWantHeld: true,
  onGoTo: true,
  onPickRepository: true,
  onAddRepository: true,
  onNewProject: true,
  onRescueSlot: true,
  onBrought: true,
};

/**
 * The callbacks as ones that never change, each calling the latest the window handed in. The window makes new ones on every draw, and
 * the deck is drawn only when its data changes, so a callback it kept would be one from an older draw. One is absent while the
 * window gave none, as it was.
 */
function useSteady(props: Omit<CockpitProps, "now">): Pick<CockpitProps, Act> {
  const latest = useRef(props);
  latest.current = props;
  const given = (Object.keys(ACTS) as Act[]).filter((one) => props[one] !== undefined);
  return useMemo(
    () =>
      Object.fromEntries(
        given.map((one) => [one, (...args: unknown[]) => (latest.current[one] as ((...args: unknown[]) => unknown) | undefined)?.(...args)]),
      ) as Pick<CockpitProps, Act>,
    [given.join()],
  );
}

export function CommandCentral(props: Omit<CockpitProps, "now">) {
  const acts = useSteady(props);
  const { state, picked, bring, bayActs } = props;
  // The deck reads ages in whole minutes, so it reads the minute: a clock that moves each second would draw all of it each second.
  return <DeckBody state={state} now={useNow(AGE_MS)} picked={picked} bring={bring} bayActs={bayActs} {...acts} />;
}
