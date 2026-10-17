// Release on a Session's flight controls: a confirmation that names each thing the Session holds and asks where it goes, and, once Fleet
// has answered, what moved and what stayed in place of the tiles. Each thing is a checkbox, all ticked when it opens. The owner, 16 Oct 2026: "This button will let me force release
// everything in the ledger of that session. Agents sometimes can't do this but from Bridge we should be able to make it happen. It would be
// cool if I could select another session to transfer what it holds to." What it acts on and the confirm key's words are `session-tiles.ts`.

import { useState } from "react";
import type { ReactNode } from "react";
import { GitBranch, GitPullRequest, KeyRound, Link } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { Button, Checkbox, Dialog, Select, Tooltip } from "@armada/components";
import type { HeldRow, HoldsReleased, Session } from "@armada/screens/src/draft/sessions";

import { heldKey, releaseKeyOf, type Held } from "./session-tiles";

const GLYPH: Record<Held["kind"], LucideIcon> = { slot: KeyRound, branch: GitBranch, pr: GitPullRequest, need: Link };
const SAID: Record<Held["kind"], string> = { slot: "Slot", branch: "Branch", pr: "Pull request", need: "Need" };

const kindOf = (kind: string): Held["kind"] | undefined => (Object.hasOwn(GLYPH, kind) ? (kind as Held["kind"]) : undefined);

function Mark({ kind, said }: { kind: Held["kind"]; said: string }) {
  const Glyph = GLYPH[kind];
  return (
    <Tooltip label={said}>
      <span className="deck-holds__mark" role="img" aria-label={said}>
        <Glyph size={12} strokeWidth={2} aria-hidden="true" />
      </span>
    </Tooltip>
  );
}

function Row({ kind, said, children }: { kind: Held["kind"]; said: string; children: ReactNode }) {
  return (
    <li className="deck-holds__row">
      <Mark kind={kind} said={said} />
      <span className="deck-holds__text">{children}</span>
    </li>
  );
}

const keyOf = heldKey;

/**
 * What the press sends: the rows ticked, given back or handed to the Session picked. Every row is ticked when it opens, the owner's
 * word of 16 Oct 2026: "If we're building this, I should be able to check what I want to release. By default everything is checked."
 * `send` resolves to whether Fleet took it.
 */
export function ReleaseHoldsDialog({
  session,
  held,
  targets,
  send,
  onCancel,
}: {
  session: Session;
  held: readonly Held[];
  targets: readonly Session[];
  send: (to: Session | undefined, rows: readonly Held[]) => Promise<boolean>;
  onCancel: () => void;
}) {
  const [to, setTo] = useState<string>("");
  const [sending, setSending] = useState(false);
  const [ticked, setTicked] = useState<ReadonlySet<string>>(() => new Set(held.map(keyOf)));
  const chosen = held.filter((one) => ticked.has(keyOf(one)));
  const tick = (key: string, on: boolean) =>
    setTicked((was) => {
      const next = new Set(was);
      if (on) next.add(key);
      else next.delete(key);
      return next;
    });
  const target = targets.find((one) => one.id === to);
  const name = session.title ?? session.address ?? session.id;
  const confirm = async () => {
    if (sending || chosen.length === 0) return;
    setSending(true);
    const done = await send(target, chosen);
    setSending(false);
    if (!done) onCancel();
  };
  return (
    <Dialog
      open
      tone="destructive"
      title={`Release what ${name} holds`}
      confirmLabel={releaseKeyOf(target, chosen.length === held.length)}
      confirmDisabled={sending || chosen.length === 0}
      onConfirm={() => void confirm()}
      onCancel={onCancel}
      field={
        <Select label="Where it goes" value={to} onChange={(event) => setTo(event.target.value)}>
          <option value="">Release</option>
          {targets.map((one) => (
            <option key={one.id} value={one.id}>
              {one.title ?? one.address ?? one.id}
            </option>
          ))}
        </Select>
      }
    >
      <ul className="deck-holds" aria-label={`What ${name} holds`}>
        {held.map((one) => (
          <li key={keyOf(one)} className="deck-holds__row">
            <Checkbox checked={ticked.has(keyOf(one))} onChange={(event) => tick(keyOf(one), event.target.checked)}>
              <Mark kind={one.kind} said={one.said} />
              <span className="deck-holds__text">{one.name}</span>
            </Checkbox>
          </li>
        ))}
      </ul>
      <p className="deck-holds__note">
        {target === undefined
          ? "Each slot ticked has its uncommitted files committed to its branch, then is freed. No branch or file is deleted."
          : `Each slot ticked moves to ${target.title ?? target.address ?? target.id} as it stands, uncommitted files included.`}{" "}
        A slot takes the branch it has checked out with it, and a running turn is stopped where a slot is ticked.
      </p>
    </Dialog>
  );
}

/**
 * Release on one ledger row: the one thing named, given back with no target. The owner, 16 Oct 2026: "I should also be able to
 * release items from the ledger panel one by one." `send` resolves to whether Fleet took it.
 */
export function ReleaseRowDialog({ held, send, onCancel }: { held: Held; send: (row: Held) => Promise<boolean>; onCancel: () => void }) {
  const [sending, setSending] = useState(false);
  const confirm = async () => {
    if (sending) return;
    setSending(true);
    const done = await send(held);
    setSending(false);
    if (!done) onCancel();
  };
  return (
    <Dialog open tone="destructive" title={`Release ${held.name}`} confirmLabel="Release" confirmDisabled={sending} onConfirm={() => void confirm()} onCancel={onCancel}>
      <ul className="deck-holds" aria-label="What is released">
        <Row kind={held.kind} said={held.said}>
          {held.name}
        </Row>
      </ul>
      <p className="deck-holds__note">
        {held.kind === "slot"
          ? "Its uncommitted files are committed to its branch, then the slot is freed. It takes the branch it has checked out with it, and a running turn is stopped."
          : "No branch or file is deleted."}
      </p>
    </Dialog>
  );
}

/** What came back from Fleet, in place of the tiles until Close. `nameOfRow` names a row as the confirmation named it, read off the Session as it stood before the release. */
export function HoldsReleasedBox({
  released,
  to,
  unticked = [],
  nameOfRow,
  onClose,
}: {
  released: HoldsReleased;
  to: Session | undefined;
  /** What the person left unticked, which is still the Session's. */
  unticked?: readonly Held[];
  nameOfRow: (row: HeldRow) => string;
  onClose: () => void;
}) {
  const moved = released.moved.flatMap((row) => {
    const kind = kindOf(row.kind);
    return kind === undefined ? [] : [{ row, kind }];
  });
  const kept = [
    ...released.kept.flatMap((row) => {
      const kind = kindOf(row.kind);
      return kind === undefined ? [] : [{ row, kind }];
    }),
    ...unticked.map((one) => ({ row: { kind: one.kind, target: one.target, why: "Not ticked" }, kind: one.kind })),
  ];
  const where = to === undefined ? "Released" : `Handed to ${to.title ?? to.address ?? to.id}`;
  return (
    <div className="deck-controls__block deck-holds-done" data-subject="session" role="status">
      <div className="deck-holds-done__lists">
        {moved.length === 0 ? null : (
          <section aria-label={where}>
            <h3 className="deck-holds-done__head">{where}</h3>
            <ul className="deck-holds">
              {moved.map(({ row, kind }) => (
                <Row key={`${row.kind}:${row.target}`} kind={kind} said={SAID[kind]}>
                  {nameOfRow(row)}
                  {row.saved === undefined || row.saved.files.length === 0 ? null : (
                    <span className="deck-holds__why">{`Committed ${row.saved.files.join(", ")} to ${row.saved.branch}`}</span>
                  )}
                </Row>
              ))}
            </ul>
          </section>
        )}
        {kept.length === 0 ? null : (
          <section aria-label="Still held">
            <h3 className="deck-holds-done__head">Still held</h3>
            <ul className="deck-holds">
              {kept.map(({ row, kind }) => (
                <Row key={`${row.kind}:${row.target}`} kind={kind} said={SAID[kind]}>
                  {nameOfRow(row)}
                  <span className="deck-holds__why">{row.why}</span>
                </Row>
              ))}
            </ul>
          </section>
        )}
      </div>
      <Button variant="secondary" size="sm" onClick={onClose}>
        Close
      </Button>
    </div>
  );
}
