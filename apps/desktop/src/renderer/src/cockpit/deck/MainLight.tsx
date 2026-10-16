// Main's light on the merge queue, when a main is red: the same panel the Merge line draws for the red, on a card the light opens.
// Hovering or focusing the light opens it, a press pins it, and Esc or a press outside puts it away. The card is the Merge line's
// own band with the Merge line's own acts (`useFixChoice`), so who is fixing, who holds the claim and the ways to hand it to a Job
// or a Session are one thing in both places. Under it is the checkout, while a served repository's is held behind main, and a green
// main with one has a card too. #2301.

import { FolderGit2 } from "lucide-react";
import { MainRedBand, Tooltip } from "@armada/components";
import { recentSessionsOf, type MergeLineView } from "@armada/screens";

import { useFixChoice } from "../../merge-line";
import { useSessions } from "../../sessions-draft";
import { LightCard } from "./LightCard";
import { checkoutSaid, YourCheckout, type Held } from "./YourCheckout";
import type { CockpitProps } from "./props";
import { lightPinned, select, selOfSubject, useDeck } from "./store";
import type { Ship } from "./ship";

type Red = { view: MergeLineView & { hub: NonNullable<MergeLineView["hub"]> }; name: string };

/** One red repository's band, with the Sessions the work can be sent to and the acts the Merge line sends. */
function RedBand({ red, props, ship, named, away }: { red: Red; props: CockpitProps; ship: Ship; named: boolean; away: () => void }) {
  const sessions = useSessions();
  const main = red.view.hub.main;
  const culprit = main?.state === "red" ? main.red.merge?.session : undefined;
  const fix = useFixChoice(red.view.root, props.onFix, props.onSendToSession);
  if (main?.state !== "red") return null;
  // A Job or Session the deck lists opens on its stage; any other opens on its own page.
  const open = (kind: "job" | "session", id: string, elsewhere: (id: string) => void) => {
    const held = ship.subjects.find((one) => one.kind === kind && one.id === id);
    if (held === undefined) elsewhere(id);
    else select(selOfSubject(held));
  };
  return (
    <section className="deck-main-card__repo" aria-label={red.name}>
      {named ? <h3 className="deck-main-card__name">{red.name}</h3> : null}
      <MainRedBand
        main={main}
        recent={red.view.hub.recent}
        recentSessions={recentSessionsOf(sessions, culprit)}
        onOpenLink={props.onOpenLink}
        onOpenJob={(id) => open("job", id, props.onOpen)}
        onOpenSession={(id) => open("session", id, props.onOpenSession)}
        {...(fix === undefined
          ? {}
          : {
              onFix: (choice) => {
                // A new Session is made in the Dispatch panel, which opens over the card.
                if (choice.kind === "newSession") away();
                return fix(choice);
              },
            })}
      />
    </section>
  );
}

export function MainLight({ props, ship, reds, held, said }: { props: CockpitProps; ship: Ship; reds: readonly Red[]; held: readonly Held[]; said: string }) {
  const summoned = useDeck((deck) => deck.light === "checkout");
  const mark = held.length === 0 ? undefined : checkoutSaid(held);
  return (
    <LightCard
      said={said}
      mark={{ "data-main": true }}
      summoned={summoned}
      onSummoned={lightPinned}
      to=".deck-checkout"
      chip={
        <>
          <span className="deck-chip__dot" data-tone={reds.length > 0 ? "bad" : "calm"} aria-hidden="true" />
          {mark === undefined ? null : (
            <Tooltip label={mark}>
              <span className="deck-chip__checkout" role="img" aria-label={mark}>
                <FolderGit2 size={12} aria-hidden="true" />
              </span>
            </Tooltip>
          )}
          <span>main</span>
        </>
      }
    >
      {(away) => (
        <>
          {reds.length > 0 ? null : (
            <div className="deck-main-card__state" role="status" aria-label="Main is green">
              <span className="deck-chip__dot" data-tone="calm" aria-hidden="true" />
              Main is green
            </div>
          )}
          {reds.map((red) => (
            <RedBand key={red.view.root} red={red} props={props} ship={ship} named={reds.length > 1} away={away} />
          ))}
          {held.map((one) => (
            <YourCheckout key={one.root} held={one} named={held.length > 1} />
          ))}
        </>
      )}
    </LightCard>
  );
}
