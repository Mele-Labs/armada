// Release on a Session's flight controls: a confirmation that names each thing the Session holds and asks where it goes, and, once Fleet
// has answered, what moved and what stayed in place of the tiles. The owner, 16 Oct 2026: "This button will let me force release
// everything in the ledger of that session. Agents sometimes can't do this but from Bridge we should be able to make it happen. It would be
// cool if I could select another session to transfer what it holds to." What it acts on and the confirm key's words are `session-tiles.ts`.

import { useState } from "react";
import type { ReactNode } from "react";
import { Box, GitBranch, GitPullRequest, KeyRound, Link } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { Button, Dialog, Select, Tooltip } from "@armada/components";
import type { HeldRow, HoldsReleased, Session } from "@armada/screens/src/draft/sessions";

import { releaseKeyOf, type Held } from "./session-tiles";

const GLYPH: Record<Held["kind"], LucideIcon> = { slot: KeyRound, branch: GitBranch, pr: GitPullRequest, job: Box, need: Link };
const SAID: Record<Held["kind"], string> = { slot: "Slot", branch: "Branch", pr: "Pull request", job: "Job", need: "Need" };

const kindOf = (kind: string): Held["kind"] | undefined => (Object.hasOwn(GLYPH, kind) ? (kind as Held["kind"]) : undefined);

function Row({ kind, said, children }: { kind: Held["kind"]; said: string; children: ReactNode }) {
  const Glyph = GLYPH[kind];
  return (
    <li className="deck-holds__row">
      <Tooltip label={said}>
        <span className="deck-holds__mark" role="img" aria-label={said}>
          <Glyph size={12} strokeWidth={2} aria-hidden="true" />
        </span>
      </Tooltip>
      <span className="deck-holds__text">{children}</span>
    </li>
  );
}

/** What the press sends: give back, or hand to the Session picked. `send` resolves to whether Fleet took it. */
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
  send: (to: Session | undefined) => Promise<boolean>;
  onCancel: () => void;
}) {
  const [to, setTo] = useState<string>("");
  const [sending, setSending] = useState(false);
  const target = targets.find((one) => one.id === to);
  const name = session.title ?? session.address ?? session.id;
  const confirm = async () => {
    if (sending) return;
    setSending(true);
    const done = await send(target);
    setSending(false);
    if (!done) onCancel();
  };
  return (
    <Dialog
      open
      tone="destructive"
      title={`Release everything ${name} holds`}
      confirmLabel={releaseKeyOf(target)}
      confirmDisabled={sending}
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
          <Row key={`${one.kind}:${one.name}`} kind={one.kind} said={one.said}>
            {one.name}
          </Row>
        ))}
      </ul>
      <p className="deck-holds__note">
        {target === undefined
          ? "Each slot's uncommitted files are committed to its branch, then the slot is freed. A running turn is stopped. No branch or file is deleted."
          : `Each slot moves to ${target.title ?? target.address ?? target.id} as it stands, uncommitted files included. A running turn is stopped.`}
      </p>
    </Dialog>
  );
}

/** What came back from Fleet, in place of the tiles until Close. */
/** `nameOfRow` names a row as the confirmation named it, read off the Session as it stood before the release. */
export function HoldsReleasedBox({
  released,
  to,
  nameOfRow,
  onClose,
}: {
  released: HoldsReleased;
  to: Session | undefined;
  nameOfRow: (row: HeldRow) => string;
  onClose: () => void;
}) {
  const moved = released.moved.flatMap((row) => {
    const kind = kindOf(row.kind);
    return kind === undefined ? [] : [{ row, kind }];
  });
  const kept = released.kept.flatMap((row) => {
    const kind = kindOf(row.kind);
    return kind === undefined ? [] : [{ row, kind }];
  });
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
          <section aria-label="Stayed">
            <h3 className="deck-holds-done__head">Stayed</h3>
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
