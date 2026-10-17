// A Session's Loadout in the panel over the work area, the one a Check opens in: drawn at the window's level beside
// that one, so opening either lets the other go (`useAsked`). Read when it opens, and again whenever the record's
// summary moves, since a `session.changed` is when `get_session_loadout` says to read it again.

import { useEffect, useState } from "react";
import { LoadoutSheet, type LoadoutRead } from "@armada/components";
import type { LoadoutSummary } from "@armada/protocol";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { useSessions, useSessionsDraft } from "./sessions-draft";
import { useAtFloor } from "@armada/shell";

/** What moves when the Session reports a new list or a new use. */
const summaryKey = (summary: LoadoutSummary | undefined): string =>
  summary === undefined ? "" : `${summary.knows}/${summary.told}/${summary.latest_turn?.started_at ?? ""}/${summary.latest_turn?.used.length ?? 0}`;

function useLoadoutRead(session: Session, read: SessionsDraft["loadout"] | undefined): LoadoutRead {
  const latest = session.loadout?.latest_turn;
  const [got, setGot] = useState<LoadoutRead>({ state: "reading" });
  const key = summaryKey(session.loadout);
  const { id } = session;
  useEffect(() => {
    if (read === undefined) {
      setGot({ state: "failed" });
      return;
    }
    let live = true;
    void read(id).then((loadout) => {
      if (live) setGot(loadout === undefined ? { state: "failed" } : { state: "read", loadout });
    });
    return () => {
      live = false;
    };
  }, [read, id, key]);
  useEffect(() => setGot({ state: "reading" }), [id]);
  if (got.state === "read") return got;
  return latest === undefined ? got : { ...got, latest };
}

/** The Loadout of Session `id`, open. Nothing while that Session is not listed or reports no Loadout. */
export function LoadoutPanel({ id, onClose }: { id: string; onClose: () => void }) {
  const session = useSessions().find((one) => one.id === id);
  const draft = useSessionsDraft();
  if (session?.loadout === undefined) return null;
  return <SessionLoadout session={session} read={draft?.loadout} onClose={onClose} />;
}

function SessionLoadout({ session, read, onClose }: { session: Session; read: SessionsDraft["loadout"] | undefined; onClose: () => void }) {
  const floor = useAtFloor();
  const got = useLoadoutRead(session, read);
  return <LoadoutSheet key={session.id} open read={got} floor={floor} onClose={onClose} />;
}
