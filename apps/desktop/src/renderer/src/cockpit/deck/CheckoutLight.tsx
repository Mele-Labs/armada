// The checkout's light on the merge queue, beside main's, while a served repository's checkout is behind main and Fleet would not
// catch it up. Hovering or focusing it opens a card with what holds the checkout back and, where one press is safe, the command
// that brings it level; a press pins the card, and so does opening the call that told him. Run is the Session band's own
// runnable block, played where something runs a command in the repository, and Copy where nothing does.

import { FolderGit2, TriangleAlert } from "lucide-react";
import { CopyAct, RunnableBlock, Tooltip, type CodeRunner } from "@armada/components";
import type { MergeLineView } from "@armada/screens";

import { checkoutCall, type CheckoutCall } from "../checkout";
import { useSessionsDraft } from "../../sessions-draft";
import { LightCard } from "./LightCard";
import { lightPinned, useDeck } from "./store";

export type Held = { root: string; name: string; call: CheckoutCall };

/** The repositories whose served checkout is held, with what the card says of each. */
export function heldOf(views: readonly MergeLineView[], nameOf: (root: string) => string): Held[] {
  return views.flatMap((view) => (view.hub?.checkout === undefined ? [] : [{ root: view.root, name: nameOf(view.root), call: checkoutCall(view.root, view.hub.checkout) }]));
}

/** The command in a block with Copy, and Run where something runs it in the repository. */
function Command({ root, command }: { root: string; command: string }) {
  const run = useSessionsDraft()?.runInRepository;
  const block = (
    <pre className="armada-prose__block">
      <code>{command}</code>
    </pre>
  );
  if (run === undefined) {
    return (
      <div className="armada-prose__run">
        <div className="armada-prose__run-block">
          {block}
          <span className="armada-prose__run-acts">
            <CopyAct text={command} />
          </span>
        </div>
      </div>
    );
  }
  const runner: CodeRunner = (text, on) => run(root, text, on);
  return (
    <RunnableBlock command={command} runner={runner} copy={command}>
      {block}
    </RunnableBlock>
  );
}

function Hold({ held, named }: { held: Held; named: boolean }) {
  const { call } = held;
  return (
    <section className="deck-checkout" aria-label={held.name}>
      {named ? <h3 className="deck-main-card__name">{held.name}</h3> : null}
      <h2 className="deck-checkout__title">{call.title}</h2>
      {call.fact === "" ? null : <p className="deck-checkout__fact">{call.fact}</p>}
      {call.facts.length === 0 ? null : (
        <dl className="deck-checkout__facts" aria-label="Checkout">
          {call.facts.map((one) => (
            <div key={one.label} className="deck-checkout__row">
              <dt>{one.label}</dt>
              <dd>
                {one.lines.map((line) => (
                  <span key={line.text} className="deck-checkout__line">
                    {line.text}
                    {line.note === undefined ? null : (
                      <Tooltip label={line.note}>
                        <span role="img" aria-label={line.note} className="deck-checkout__note">
                          <TriangleAlert size={12} aria-hidden="true" />
                        </span>
                      </Tooltip>
                    )}
                  </span>
                ))}
              </dd>
            </div>
          ))}
        </dl>
      )}
      {call.command === undefined ? null : <Command root={held.root} command={call.command} />}
    </section>
  );
}

export function CheckoutLight({ held }: { held: readonly Held[] }) {
  const summoned = useDeck((deck) => deck.light === "checkout");
  const said = held.map((one) => `Checkout ${one.call.title}${held.length > 1 ? ` in ${one.name}` : ""}`).join(", ");
  return (
    <LightCard
      said={said}
      mark={{ "data-tone": "warn" }}
      summoned={summoned}
      onSummoned={lightPinned}
      chip={
        <>
          <span className="deck-chip__glyph" aria-hidden="true">
            <FolderGit2 size={12} />
          </span>
          <span>checkout</span>
        </>
      }
    >
      {() => held.map((one) => <Hold key={one.root} held={one} named={held.length > 1} />)}
    </LightCard>
  );
}
