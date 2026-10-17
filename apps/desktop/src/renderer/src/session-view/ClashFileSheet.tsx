// One file of a write clash, opened from Bays: the Session's change to it, then the other side's, each from where the two split, and
// the hunks that collide marked in both. A floating sheet beside the stage; Esc closes it and Bays is as it was.

import { useEffect, useState, useSyncExternalStore } from "react";
import { Sheet, UnifiedDiff, type ClashHolderRef, type DiffFile } from "@armada/components";
import { markCollisions } from "@armada/screens/src/clash-hunks";
import { drawnOf } from "@armada/screens/src/drawn-patch";
import type { ClashFileRead, Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { useAtFloor } from "@armada/shell";

import { OverTheApp } from "./OverTheApp";

type Open = { session: string; with: ClashHolderRef; path: string };

let open: Open | undefined;
const hear = new Set<() => void>();
const set = (next: Open | undefined) => {
  open = next;
  for (const one of hear) one();
};

/** Opens one file of the Session's clash with `other`. */
export const openClashFile = (session: string, other: ClashHolderRef, path: string) => set({ session, with: other, path });
const closeClashFile = () => set(undefined);

function useOpen(session: string): Open | undefined {
  const now = useSyncExternalStore(
    (listener) => {
      hear.add(listener);
      return () => void hear.delete(listener);
    },
    () => open,
  );
  return now?.session === session ? now : undefined;
}

const cutSaid = (drawn: number, total: number) => `The first ${drawn.toLocaleString()} of ${total.toLocaleString()} lines.`;
const fileOf = (patch: string): DiffFile | undefined => drawnOf(patch, cutSaid).files[0];

function Side({ label, file }: { label: string; file: DiffFile | undefined }) {
  return (
    <section className="sv-clash-file__side" aria-label={label}>
      <h4 className="sv-layer__group-label">{label}</h4>
      <UnifiedDiff files={file === undefined ? [] : [file]} emptyNote="No lines changed in this file." />
    </section>
  );
}

export function ClashFileSheet({ session, draft }: { session: Session; draft: SessionsDraft | undefined }) {
  const floor = useAtFloor();
  const shown = useOpen(session.id);
  const [read, setRead] = useState<{ for: Open; answer: ClashFileRead } | undefined>(undefined);
  useEffect(() => {
    if (shown === undefined || draft?.clashFile === undefined) return;
    let current = true;
    void draft.clashFile(session.id, shown.with, shown.path).then((answer) => {
      if (current) setRead({ for: shown, answer });
    });
    return () => {
      current = false;
    };
  }, [shown, draft, session.id]);
  if (shown === undefined) return null;
  const answer = read?.for === shown ? read.answer : undefined;
  const ours = session.title ?? session.id;
  const theirs = shown.with.label;
  const mine = answer?.ok === true ? fileOf(answer.ours) : undefined;
  const other = answer?.ok === true ? fileOf(answer.theirs) : undefined;
  return (
    <OverTheApp open>
    <Sheet open floating kind="session-clash-file" floor={floor} title={shown.path} closeLabel="Close" closeBinding="Esc" onClose={closeClashFile}>
      <div className="armada-pr-file-diff sv-clash-file" aria-busy={answer === undefined || undefined}>
        {answer?.ok === false ? (
          <p role="note">{answer.said}</p>
        ) : answer === undefined ? null : (
          <>
            <Side label={ours} file={mine === undefined || other === undefined ? mine : markCollisions(mine, other, `Collides with ${theirs}`)} />
            <Side label={theirs} file={other === undefined || mine === undefined ? other : markCollisions(other, mine, `Collides with ${ours}`)} />
          </>
        )}
      </div>
    </Sheet>
    </OverTheApp>
  );
}
