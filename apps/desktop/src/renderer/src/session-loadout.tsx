// A Session's Loadout on its sheet: read when the sheet opens, and again whenever the record's summary moves,
// since a `session.changed` is when `get_session_loadout` says to read it again.

import { useEffect, useState } from "react";
import { LoadoutSheet, type LoadoutRead } from "@armada/components";
import type { LoadoutSummary } from "@armada/protocol";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";
import { useAtFloor } from "@armada/shell";

/** What moves when the Session reports a new list or a new use. */
const summaryKey = (summary: LoadoutSummary | undefined): string =>
  summary === undefined ? "" : `${summary.knows}/${summary.told}/${summary.latest_turn?.started_at ?? ""}/${summary.latest_turn?.used.length ?? 0}`;

function useLoadoutRead(session: Session, open: boolean, read: SessionsDraft["loadout"]): LoadoutRead {
  const latest = session.loadout?.latest_turn;
  const [got, setGot] = useState<LoadoutRead>({ state: "reading" });
  const key = summaryKey(session.loadout);
  const { id } = session;
  useEffect(() => {
    if (!open) return;
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
  }, [open, read, id, key]);
  useEffect(() => setGot({ state: "reading" }), [id]);
  if (got.state === "read") return got;
  return latest === undefined ? got : { ...got, latest };
}

/** The Loadout sheet over a Session, drawn while `open`. */
export function SessionLoadoutSheet({ session, open, read, onClose }: { session: Session; open: boolean; read: SessionsDraft["loadout"]; onClose: () => void }) {
  const floor = useAtFloor();
  const got = useLoadoutRead(session, open, read);
  return <LoadoutSheet key={session.id} open={open} read={got} floor={floor} onClose={onClose} />;
}
