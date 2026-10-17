// A Session's Loadout in the panel over the work area, the one a Check opens in: drawn at the window's level beside
// that one, so opening either lets the other go (`useAsked`). Read when it opens, and again whenever the record's
// summary moves, since a `session.changed` is when `get_session_loadout` says to read it again. A row pressed opens
// what that item says to the Loadout's left, read once per press with `read_loadout_item`.

import { useCallback, useEffect, useState } from "react";
import { LoadoutSheet, loadoutKeyOf, type LoadoutItemRead, type LoadoutOpened, type LoadoutRead, type LoadoutRef } from "@armada/components";
import type { LoadoutSummary } from "@armada/protocol";
import type { LoadoutItemBody, Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

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
  return <SessionLoadout key={session.id} session={session} draft={draft} onClose={onClose} />;
}

/** The wire's answer as the panel draws it. */
const itemReadOf = (body: LoadoutItemBody | undefined): LoadoutItemRead =>
  body === undefined
    ? { state: "failed" }
    : body.what === "not_read"
      ? { state: "not_read", why: body.why }
      : { state: "read", format: body.format, body: body.body, ...(body.path === undefined ? {} : { path: body.path }), ...(body.truncated === true ? { truncated: true } : {}) };

/**
 * The item open beside the Loadout, read when its row is pressed. **The last press wins**: an answer for a row
 * pressed before it is dropped. An MCP server is not read, since the Loadout already lists its tools.
 */
function useOpenedItem(id: string, read: SessionsDraft["loadoutItem"] | undefined): [LoadoutOpened | undefined, (item: LoadoutRef | null) => void] {
  const [opened, setOpened] = useState<LoadoutOpened | undefined>(undefined);
  const open = useCallback(
    (item: LoadoutRef | null) => {
      if (item === null) return setOpened(undefined);
      if (item.kind === "mcp_server") return setOpened({ item });
      if (read === undefined) return setOpened({ item, read: { state: "failed" } });
      setOpened({ item, read: { state: "reading" } });
      const key = loadoutKeyOf(item);
      void read(id, item).then((body) => setOpened((now) => (now === undefined || loadoutKeyOf(now.item) !== key ? now : { item, read: itemReadOf(body) })));
    },
    [id, read],
  );
  return [opened, open];
}

function SessionLoadout({ session, draft, onClose }: { session: Session; draft: SessionsDraft | undefined; onClose: () => void }) {
  const floor = useAtFloor();
  const got = useLoadoutRead(session, draft?.loadout);
  const [opened, openItem] = useOpenedItem(session.id, draft?.loadoutItem);
  return <LoadoutSheet key={session.id} open read={got} floor={floor} onClose={onClose} {...(opened === undefined ? {} : { opened })} onOpenItem={openItem} />;
}
