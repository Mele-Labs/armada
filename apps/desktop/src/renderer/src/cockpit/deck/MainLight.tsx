// Main's light on the merge queue, when a main is red: the same panel the Merge line draws for the red, on a card the light opens.
// Hovering or focusing the light opens it, a press pins it, and Esc or a press outside puts it away. The card is the Merge line's
// own band with the Merge line's own acts (`useFixChoice`), so who is fixing, who holds the claim and the ways to hand it to a
// Session are one thing in both places. Under it is the checkout, while a served repository's is held behind main, and a green
// main with one has a card too. #2301.

import { CircleCheck, CircleX, FolderGit2, Unplug } from "lucide-react";
import { MainRedBand, Tooltip } from "@armada/components";
import { recentSessionsOf, type MergeLineView } from "@armada/screens";

import { useFixChoice } from "../../merge-line";
import { useSessions } from "../../sessions-draft";
import { ReadAgo, type Forge } from "./forge";
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
  const fix = useFixChoice(red.view.root, props.onSendToSession);
  if (main?.state !== "red") return null;
  // A Session the deck lists opens on its stage; any other opens on its own page.
  const open = (id: string, elsewhere: (id: string) => void) => {
    const held = ship.subjects.find((one) => one.kind === "session" && one.id === id);
    if (held === undefined) elsewhere(id);
    else select(selOfSubject(held));
  };
  return (
    <section className="deck-main-card__repo" aria-label={red.name}>
      {named ? <h3 className="deck-main-card__name">{red.name}</h3> : null}
      <MainRedBand
        main={main}
        recentSessions={recentSessionsOf(sessions, culprit)}
        onOpenLink={props.onOpenLink}
        onOpenSession={(id) => open(id, props.onOpenSession)}
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

/** `name` is what the chip says: `main`, or on All the repository's name, one light each. `summon`: the one light the checkout key opens. */
export function MainLight({ props, ship, reds, held, said, forge, quiet, name = "main", summon = true, calling = false }: { props: CockpitProps; ship: Ship; reds: readonly Red[]; held: readonly Held[]; said: string; forge: Forge; quiet: string | undefined; name?: string; summon?: boolean; calling?: boolean }) {
  const summoned = useDeck((deck) => deck.light === "checkout") && summon;
  const mark = held.length === 0 ? undefined : checkoutSaid(held);
  return (
    <LightCard
      said={said}
      mark={{ "data-main": true, ...(calling ? { "data-calling": "" } : {}) }}
      summoned={summoned}
      onSummoned={lightPinned}
      to=".deck-checkout"
      state={reds.length > 0 ? { Glyph: CircleX, said: "Red", tone: "var(--status-completed-failed)" } : { Glyph: CircleCheck, said: "Green", tone: "var(--status-completed-success)" }}
      chip={
        <>
          <span className="deck-chip__dot" data-tone={reds.length > 0 ? "bad" : "calm"} data-stale={quiet !== undefined || undefined} aria-hidden="true" />
          {quiet === undefined ? null : (
            <Tooltip label={quiet}>
              <span className="deck-chip__quiet" role="img" aria-label={quiet}>
                <Unplug size={12} aria-hidden="true" />
              </span>
            </Tooltip>
          )}
          {mark === undefined ? null : (
            <Tooltip label={mark}>
              <span className="deck-chip__checkout" role="img" aria-label={mark}>
                <FolderGit2 size={12} aria-hidden="true" />
              </span>
            </Tooltip>
          )}
          <span>{name}</span>
        </>
      }
    >
      {(away) => (
        <>
          {reds.map((red) => (
            <RedBand key={red.view.root} red={red} props={props} ship={ship} named={reds.length > 1} away={away} />
          ))}
          {held.map((one) => (
            <YourCheckout key={one.root} held={one} named={held.length > 1} />
          ))}
          {forge.readAt === undefined && quiet === undefined ? null : (
            <div className="deck-main-card__forge">
              {forge.readAt === undefined ? null : <span><ReadAgo at={forge.readAt} /></span>}
              {quiet === undefined ? null : (
                <span className="deck-main-card__quiet">
                  <Unplug size={12} aria-hidden="true" />
                  {quiet}
                </span>
              )}
            </div>
          )}
        </>
      )}
    </LightCard>
  );
}
