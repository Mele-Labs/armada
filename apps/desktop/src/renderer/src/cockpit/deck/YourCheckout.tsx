// "Your checkout", the section of main's card for a served repository whose checkout is behind main and Fleet would not catch it
// up: what holds it back and, where one press is safe, the command that brings it level. Run is the Session band's own runnable
// block, played where something runs a command in the repository, and Copy where nothing does. Each uncommitted path opens its
// change in place where Fleet reads it, and a second press folds it.

import { Fragment, useEffect, useRef, useState } from "react";
import { FolderGit2, GitMerge, GitPullRequestArrow, TriangleAlert } from "lucide-react";
import { Button, Chip, CopyAct, OwnerChip, RunnableBlock, Tooltip, UnifiedDiff, type CodeRunner } from "@armada/components";
import type { MergeLineView } from "@armada/screens";
import type { CheckoutDiffRead } from "@armada/screens/src/draft/sessions";

import { checkoutCall, type CheckoutCall, type Fact } from "../checkout";
import { checkoutDiffOf } from "../checkout-diff";
import { useRepositoryActs } from "../../sessions-draft";

export type Held = { root: string; name: string; call: CheckoutCall };

/** The repositories whose served checkout is held, with what the card says of each. */
export function heldOf(views: readonly MergeLineView[], nameOf: (root: string) => string): Held[] {
  return views.flatMap((view) => (view.hub?.checkout === undefined ? [] : [{ root: view.root, name: nameOf(view.root), call: checkoutCall(view.root, view.hub.checkout) }]));
}

/** The command in a block with Copy, and Run where something runs it in the repository. */
function Command({ root, command }: { root: string; command: string }) {
  const run = useRepositoryActs().runInRepository;
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

/** The uncommitted files as a pull request: committed to a branch, opened, auto-merge on. Pending until Fleet answers with its number. */
function SendAsPullRequest({ root }: { root: string }) {
  const { sendCheckoutAsPullRequest: send, checkoutSendFailed: failed } = useRepositoryActs();
  const [pending, setPending] = useState(false);
  const stop = useRef<(() => void) | undefined>(undefined);
  useEffect(() => () => stop.current?.(), []);
  if (send === undefined) return null;
  const free = () => {
    stop.current?.();
    stop.current = undefined;
    setPending(false);
  };
  return (
    <span className="deck-checkout__send">
    <Tooltip label="Commit to a branch, open a pull request, auto-merge on">
      <Button
        variant="ghost"
        size="sm"
        pending={pending}
        onClick={() => {
          setPending(true);
          // Sent, the button waits for the hold to carry the pull request and the card to swap it for the chip; a refusal, or the send failing afterwards, frees it.
          stop.current?.();
          stop.current = failed?.(root, free);
          void send(root).then((sent) => sent === undefined && free(), free);
        }}
      >
        <GitPullRequestArrow size={12} aria-hidden="true" />
        Pull request
      </Button>
    </Tooltip>
    </span>
  );
}

/** The pull request opened from the files: its chip, and auto-merge as a glyph that a tooltip names. A row of the facts' own grid. */
function PullRequestRow({ pullRequest }: { pullRequest: NonNullable<CheckoutCall["pullRequest"]> }) {
  const chip = { kind: "pull_request", number: pullRequest.number } as const;
  return (
    <>
      <dt>Pull request</dt>
      <dd>
        <span className="deck-checkout__line">
          <OwnerChip chip={chip}>
            <Chip chip={chip} />
          </OwnerChip>
          {pullRequest.autoMerge ? (
            <Tooltip label="Auto-merge on">
              <span role="img" aria-label="Auto-merge on" className="deck-checkout__auto">
                <GitMerge size={12} aria-hidden="true" />
              </span>
            </Tooltip>
          ) : null}
        </span>
      </dd>
    </>
  );
}

/** Lines a row shows before it scrolls. */
const SHOWN = 6;

/** The label of the row whose lines are files Fleet can read the change of. */
const UNCOMMITTED = "Uncommitted";

function Note({ note }: { note: string | undefined }) {
  if (note === undefined) return null;
  return (
    <Tooltip label={note}>
      <span role="img" aria-label={note} className="deck-checkout__note">
        <TriangleAlert size={12} aria-hidden="true" />
      </span>
    </Tooltip>
  );
}

/** An uncommitted path that opens its change under it when pressed, and folds it when pressed again. Read again each time it opens. */
function Change({ root, line, read }: { root: string; line: Fact["lines"][number]; read: (root: string, path: string) => Promise<CheckoutDiffRead> }) {
  const [open, setOpen] = useState(false);
  const [answer, setAnswer] = useState<CheckoutDiffRead | undefined>(undefined);
  const here = useRef(true);
  useEffect(() => {
    here.current = true;
    return () => {
      here.current = false;
    };
  }, []);
  useEffect(() => {
    if (!open) return;
    let current = true;
    setAnswer(undefined);
    void read(root, line.text).then((done) => {
      if (current && here.current) setAnswer(done);
    });
    return () => {
      current = false;
    };
  }, [open, read, root, line.text]);
  return (
    <span className="deck-checkout__change">
      <span className="deck-checkout__line">
        <Tooltip label={open ? "Hide the change" : "Show the change"}>
          <button type="button" className="deck-checkout__path" aria-expanded={open} onClick={() => setOpen(!open)}>
            {line.text}
          </button>
        </Tooltip>
        <Note note={line.note} />
      </span>
      {open ? (
        <div className="deck-checkout__diff">
          <UnifiedDiff {...checkoutDiffOf(answer)} />
        </div>
      ) : null}
    </span>
  );
}

function Line({ root, fact, line }: { root: string; fact: Fact; line: Fact["lines"][number] }) {
  const read = useRepositoryActs().diffInRepository;
  if (fact.label === UNCOMMITTED && read !== undefined) return <Change root={root} line={line} read={read} />;
  return (
    <span className="deck-checkout__line">
      {line.text}
      <Note note={line.note} />
    </span>
  );
}

/** A row's lines: all of them, or past `SHOWN` in a box that scrolls inside the card. Clashing lines come first. */
function Lines({ root, fact }: { root: string; fact: Fact }) {
  if (fact.lines.length <= SHOWN) return <>{fact.lines.map((line) => <Line key={line.text} root={root} fact={fact} line={line} />)}</>;
  return (
    <div className="deck-checkout__files" role="group" aria-label={`${fact.label} files`} tabIndex={0}>
      {fact.lines.map((line) => (
        <Line key={line.text} root={root} fact={fact} line={line} />
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
      {call.facts.length === 0 && call.pullRequest === undefined ? null : (
        <dl className="deck-checkout__rows mono" aria-label="Checkout">
          {call.facts.map((one) => (
            <Fragment key={one.label}>
              <dt>{one.label}</dt>
              <dd>
                <Lines root={held.root} fact={one} />
              </dd>
            </Fragment>
          ))}
          {call.pullRequest === undefined ? null : <PullRequestRow pullRequest={call.pullRequest} />}
        </dl>
      )}
      {call.command === undefined && call.sendable === undefined ? null : (
        <div className="deck-checkout__command">
          {call.command === undefined ? null : (
            <div className="deck-checkout__run">
              <Command root={held.root} command={call.command} />
            </div>
          )}
          {call.sendable === undefined ? null : <SendAsPullRequest root={held.root} />}
        </div>
      )}
    </section>
  );
}
