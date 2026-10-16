// "Your checkout", the section of main's card for a served repository whose checkout is behind main and Fleet would not catch it
// up: what holds it back and, where one press is safe, the command that brings it level. Run is the Session band's own runnable
// block, played where something runs a command in the repository, and Copy where nothing does.

import { Fragment } from "react";
import { FolderGit2, TriangleAlert } from "lucide-react";
import { CopyAct, RunnableBlock, Tooltip, type CodeRunner } from "@armada/components";
import type { MergeLineView } from "@armada/screens";

import { checkoutCall, type CheckoutCall, type Fact } from "../checkout";
import { useSessionsDraft } from "../../sessions-draft";

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

/** Lines a row shows before it scrolls. */
const SHOWN = 6;

function Line({ line }: { line: Fact["lines"][number] }) {
  return (
    <span className="deck-checkout__line">
      {line.text}
      {line.note === undefined ? null : (
        <Tooltip label={line.note}>
          <span role="img" aria-label={line.note} className="deck-checkout__note">
            <TriangleAlert size={12} aria-hidden="true" />
          </span>
        </Tooltip>
      )}
    </span>
  );
}

/** A row's lines: all of them, or past `SHOWN` in a box that scrolls inside the card. Clashing lines come first. */
function Lines({ fact }: { fact: Fact }) {
  if (fact.lines.length <= SHOWN) return <>{fact.lines.map((line) => <Line key={line.text} line={line} />)}</>;
  return (
    <div className="deck-checkout__files" role="group" aria-label={`${fact.label} files`} tabIndex={0}>
      {fact.lines.map((line) => (
        <Line key={line.text} line={line} />
      ))}
    </div>
  );
}

/** What the chip's mark and the light say of the holds: `Checkout 880 commits behind main`, and the repository after each where there are several. */
export function checkoutSaid(held: readonly Held[]): string {
  return held.map((one) => `Checkout ${one.call.title}${held.length > 1 ? ` in ${one.name}` : ""}`).join(", ");
}

/** One repository's section: a band in the warning hue like the red band's, then its reason and rows, then the command. */
export function YourCheckout({ held, named }: { held: Held; named: boolean }) {
  const { call } = held;
  return (
    <section className="deck-checkout" aria-label={named ? `Your checkout in ${held.name}` : "Your checkout"} tabIndex={-1}>
      <div className="deck-checkout__head">
        <span className="deck-checkout__glyph" aria-hidden="true">
          <FolderGit2 size={12} />
        </span>
        <span className="deck-checkout__heading">Your checkout</span>
        {named ? <span className="deck-checkout__repo mono">{held.name}</span> : null}
      </div>
      <p className="deck-checkout__title">{call.title}</p>
      {call.fact === "" ? null : <p className="deck-checkout__fact">{call.fact}</p>}
      {call.facts.length === 0 ? null : (
        <dl className="deck-checkout__rows mono" aria-label="Checkout">
          {call.facts.map((one) => (
            <Fragment key={one.label}>
              <dt>{one.label}</dt>
              <dd>
                <Lines fact={one} />
              </dd>
            </Fragment>
          ))}
        </dl>
      )}
      {call.command === undefined ? null : (
        <div className="deck-checkout__command">
          <Command root={held.root} command={call.command} />
        </div>
      )}
    </section>
  );
}
