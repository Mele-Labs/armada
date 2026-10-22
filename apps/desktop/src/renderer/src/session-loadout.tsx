// A Session's Loadout in the panel over the work area, the one a Check opens in: drawn at the window's level beside
// that one, so opening either lets the other go (`useAsked`). Read when it opens, and again whenever the record's
// summary moves, since a `session.changed` is when `get_session_loadout` says to read it again. A row pressed opens
// what that item says to the Loadout's left, read once per press with `read_loadout_item`. A hosted Job's MCP servers
// are asked of its process while the panel is open, since the process pushes no change. Kit's inventory is read when
// the panel opens, so a skill Kit holds offers Edit, which opens its SKILL.md in the file panel, and one of the
// person's own offers Remove, then Restore from the Trash.

import { useCallback, useEffect, useRef, useState } from "react";
import { LoadoutSheet, loadoutKeyOf, type LoadoutItemRead, type LoadoutOpened, type LoadoutRead, type LoadoutRef, type LoadoutSkillAct, type LoadoutSkillActs, type McpGroup, type McpServerRow, type SkillGroup } from "@armada/components";
import type { KitInventory, LoadoutSummary, McpServerAct, McpServerState, SessionMcpServer } from "@armada/protocol";
import { refusalWords, whyNotOpenedLink } from "@armada/screens";
import type { LoadoutItemBody, Session, SessionsDraft } from "@armada/screens/src/draft/sessions";
import { skillActsByKit } from "@armada/screens/src/loadout-kit";
import { kitChanged } from "@armada/screens/src/manifest-kit";

import { openLink, readKitInventory, removeKitPiece, restoreKitPiece } from "./commands";
import { openInPanel } from "./shared-panel/store";
import { useSessions, useSessionsDraft } from "./sessions-draft";
import { useAtFloor } from "@armada/shell";

/** The group the Loadout opens on: typing `/mcp` in a hosted Job opens its MCP servers. */
export type LoadoutAt = "mcp";

/** How often an open Loadout asks a hosted Job's process for its MCP servers again. */
export const MCP_EVERY_MS = 5_000;

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

/** The Loadout of Session `id`, open. Nothing while that Session is not listed, or reports no Loadout and has no MCP servers to read. */
export function LoadoutPanel({ id, at, onClose }: { id: string; at?: LoadoutAt | undefined; onClose: () => void }) {
  const session = useSessions().find((one) => one.id === id);
  const draft = useSessionsDraft();
  if (session === undefined || (session.loadout === undefined && mcpReadOf(session, draft) === undefined)) return null;
  return <SessionLoadout key={session.id} session={session} draft={draft} at={at} onClose={onClose} />;
}

/** The read of a hosted Job's MCP servers. A terminal Job's come from its agent's report alone. */
const mcpReadOf = (session: Session, draft: SessionsDraft | undefined) => (session.terminal === true ? undefined : draft?.mcp);

/** A refusal is drawn while the server still stands where it stood when refused. */
type Refused = { said: string; state: McpServerState };

/**
 * A hosted Job's MCP servers, read when the panel opens and every `MCP_EVERY_MS` while it is, and after each act. **An
 * act's answer wins**: a read that answers while an act is out, or was sent before one that landed, is dropped, since the
 * process reports a server mid-reconnect until the act settles. A sign-in page
 * opens in the person's browser, and the row then offers Reconnect while the server still needs sign-in.
 */
function useMcpGroup(session: Session, draft: SessionsDraft | undefined): McpGroup | undefined {
  const read = mcpReadOf(session, draft);
  const act = draft?.actOnMcp;
  const { id } = session;
  const [servers, setServers] = useState<readonly SessionMcpServer[] | undefined>(undefined);
  const [said, setSaid] = useState<string | undefined>(undefined);
  const [acting, setActing] = useState<ReadonlyMap<string, McpServerAct>>(new Map());
  const [refused, setRefused] = useState<ReadonlyMap<string, Refused>>(new Map());
  const [signedIn, setSignedIn] = useState<ReadonlySet<string>>(new Set());
  const asked = useRef(0);
  const landed = useRef(0);
  const out = useRef(0);
  const live = useRef(true);
  const lands = useCallback((seq: number) => {
    if (!live.current || out.current > 0 || seq < landed.current) return false;
    landed.current = seq;
    return true;
  }, []);
  useEffect(() => {
    live.current = true;
    if (read === undefined) return;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const once = () => {
      const seq = (asked.current += 1);
      void read(id).then((got) => {
        if (!live.current) return;
        if (lands(seq)) {
          if (got.ok) setServers(got.servers);
          setSaid(got.ok ? undefined : got.said);
        }
        timer = setTimeout(once, MCP_EVERY_MS);
      });
    };
    once();
    return () => {
      live.current = false;
      clearTimeout(timer);
    };
  }, [read, id, lands]);
  const onAct = useCallback(
    (server: string, what: McpServerAct) => {
      if (act === undefined) return;
      const was = servers?.find((one) => one.name === server)?.state ?? "failed";
      out.current += 1;
      setActing((now) => new Map(now).set(server, what));
      setRefused((now) => without(now, server));
      void act(id, server, what).then(async (got) => {
        out.current -= 1;
        if (!live.current) return;
        setActing((now) => without(now, server));
        if (!got.ok) return setRefused((now) => new Map(now).set(server, { said: got.said, state: was }));
        landed.current = asked.current;
        setServers(got.servers);
        if (got.signIn === undefined) return;
        setSignedIn((now) => new Set(now).add(server));
        const why = whyNotOpenedLink(await openLink(got.signIn.url));
        const after = got.servers.find((one) => one.name === server)?.state ?? was;
        if (why !== null && live.current) setRefused((now) => new Map(now).set(server, { said: why, state: after }));
      });
    },
    [act, id, servers],
  );
  if (read === undefined) return undefined;
  const rows = servers?.map((one): McpServerRow => {
    const out = acting.get(one.name);
    const no = refused.get(one.name);
    return {
      ...one,
      ...(out === undefined ? {} : { acting: out }),
      ...(no === undefined || no.state !== one.state ? {} : { refused: no.said }),
      ...(one.state === "needs_sign_in" && signedIn.has(one.name) ? { signedIn: true as const } : {}),
    };
  });
  return { ...(rows === undefined ? {} : { servers: rows }), ...(said === undefined ? {} : { said }), onAct };
}

function without<V>(map: ReadonlyMap<string, V>, key: string): ReadonlyMap<string, V> {
  if (!map.has(key)) return map;
  const next = new Map(map);
  next.delete(key);
  return next;
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

/** A skill removed from this Mac by its row: where its folder went in the Trash, or `null` where nothing can bring it back. */
type Gone = ReadonlyMap<string, string | null>;

/**
 * Kit's acts on the skills it holds, matched against its inventory, read once as the panel opens and again off each
 * act's answer. **Edit opens the SKILL.md in the file panel, which takes the Loadout's place**; where no window hosts
 * that panel, the editor opens it. A skill removed keeps its row, and Restore sends the Trash's place for it back.
 */
function useSkillGroup(read: LoadoutRead, onClose: () => void): SkillGroup | undefined {
  const [setup, setSetup] = useState<KitInventory | undefined>(undefined);
  const [gone, setGone] = useState<Gone>(new Map());
  const [acting, setActing] = useState<ReadonlyMap<string, "remove" | "restore">>(new Map());
  const [refused, setRefused] = useState<ReadonlyMap<string, string>>(new Map());
  const live = useRef(true);
  useEffect(() => {
    live.current = true;
    void readKitInventory().then((got) => {
      if (live.current && got.ok) setSetup(got.setup);
    });
    return () => {
      live.current = false;
    };
  }, []);
  if (setup === undefined || read.state !== "read") return undefined;
  const matched = skillActsByKit(read.loadout.items, setup);
  const rows: Record<string, LoadoutSkillActs> = {};
  for (const key of new Set([...Object.keys(matched), ...gone.keys()])) {
    const trashed = gone.get(key);
    const out = acting.get(key);
    const no = refused.get(key);
    rows[key] = {
      ...(gone.has(key) ? { removed: { restorable: trashed !== null && trashed !== undefined } } : matched[key]!.acts),
      ...(out === undefined ? {} : { acting: out }),
      ...(no === undefined ? {} : { refused: no }),
    };
  }
  const settled = (key: string, said: string | undefined) => {
    setActing((now) => without(now, key));
    setRefused((now) => (said === undefined ? without(now, key) : new Map(now).set(key, said)));
  };
  const onAct = (item: LoadoutRef, act: LoadoutSkillAct) => {
    const key = loadoutKeyOf(item);
    if (act === "edit") {
      const path = matched[key]?.acts.edits;
      if (path === undefined) return;
      if (!openInPanel({ kind: "file", path, from: "kit" })) return void window.armada.openKitFile(path, "editor");
      return onClose();
    }
    const body = act === "remove" ? matched[key]?.remove : undefined;
    const trashed = act === "restore" ? gone.get(key) : undefined;
    if (body === undefined && (trashed === undefined || trashed === null)) return;
    setActing((now) => new Map(now).set(key, act));
    if (body !== undefined) {
      void removeKitPiece(body).then((got) => {
        if (!live.current) return;
        settled(key, got.ok ? undefined : refusalWords(got.outcome));
        if (!got.ok) return;
        setSetup(got.removed.setup);
        setGone((now) => new Map(now).set(key, got.removed.trashed));
        kitChanged();
      });
      return;
    }
    void restoreKitPiece({ trashed: trashed! }).then((got) => {
      if (!live.current) return;
      settled(key, got.ok ? undefined : refusalWords(got.outcome));
      if (!got.ok) return;
      setSetup(got.setup);
      setGone((now) => without(now, key));
      kitChanged();
    });
  };
  return { rows, onAct };
}

function SessionLoadout({ session, draft, at, onClose }: { session: Session; draft: SessionsDraft | undefined; at: LoadoutAt | undefined; onClose: () => void }) {
  const floor = useAtFloor();
  const got = useLoadoutRead(session, draft?.loadout);
  const [opened, openItem] = useOpenedItem(session.id, draft?.loadoutItem);
  const mcp = useMcpGroup(session, draft);
  const skills = useSkillGroup(got, onClose);
  return (
    <LoadoutSheet
      key={session.id}
      open
      read={got}
      floor={floor}
      onClose={onClose}
      {...(opened === undefined ? {} : { opened })}
      onOpenItem={openItem}
      {...(mcp === undefined ? {} : { mcp })}
      {...(at === undefined ? {} : { at })}
      {...(skills === undefined ? {} : { skills })}
    />
  );
}
