// The flight controls' tiles while a Session is open on the stage: the same block, size and keys, `1` to `5`, as the six usual
// ones, with the Session's own acts. What each does and why one is off is in `session-tiles.ts`. A refusal from Fleet is the
// toast the Session's composer shows, since the same `draft.send` carries it. **Report** swaps the tiles for a box (`ReportBox`);
// sending starts a Session in Armada's repository where Fleet serves it (`reportRoot`), tells it the owner's words and what Bridge holds about this one
// (`report-brief.ts`), and opens it on the stage. The owner's ask of 15 Oct 2026. **Release** opens a confirmation naming what the Session
// holds, and what Fleet answers stands in place of the tiles until Close (`ReleaseHolds.tsx`).

import { useRef, useState } from "react";
import { Bug, FoldVertical, MessageCircleMore, Orbit, PlaneLanding, Unplug } from "lucide-react";
import { Kbd, Tooltip } from "@armada/components";
import type { HoldsReleased, Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { useSessionsSaid } from "../../sessions-draft";
import { useFleetEvents } from "../fleet-events";
import { setAutoLand, useAutoLand } from "./auto-land";
import { askForUpdate } from "./acts";
import { HoldsReleasedBox, ReleaseHoldsDialog } from "./ReleaseHolds";
import { ReportBox } from "./ReportBox";
import { reportBrief } from "./report-brief";
import { COMPACT, LAND_OFF_TEXT, LAND_ON_TEXT, SESSION_TILES, handTargets, heldOf, nameOfHeldRow, offeredOf, plain, type SessionTileId } from "./session-tiles";
import { select, setView } from "./store";

const GLYPH = { ask: MessageCircleMore, land: PlaneLanding, compact: FoldVertical, report: Bug, map: Orbit, release: Unplug } as const;

/** The repository the report is about: its root and its name where it has one. */
export type ReportRepository = { root: string; name?: string };

/** `startIn` is the root a report's new Session starts in (`reportRoot`); absent, the reported Session's own. */
export function SessionTiles({ session, draft, repository, startIn }: { session: Session; draft: SessionsDraft; repository: ReportRepository | undefined; startIn?: string }) {
  const [pending, setPending] = useState<SessionTileId | undefined>();
  const [reporting, setReporting] = useState(false);
  const [releasing, setReleasing] = useState(false);
  /** What Fleet answered, with the Session as it stood before, which names each row. */
  const [released, setReleased] = useState<{ answer: HoldsReleased; to: Session | undefined; before: Session } | undefined>();
  /** The Session a report started, kept while its message is refused so a second press tells the same one. */
  const started = useRef<string | undefined>(undefined);
  const events = useFleetEvents();
  const refused = useSessionsSaid();
  const on = useAutoLand(session.id);
  const { off, compact } = offeredOf(session, draft, repository?.root);
  const sent = async (text: string): Promise<boolean> => (await draft.send(session.id, plain(text))) !== false;
  const run = async (tile: SessionTileId, act: () => Promise<void>) => {
    if (pending !== undefined) return;
    setPending(tile);
    try {
      await act();
    } finally {
      setPending(undefined);
    }
  };
  const press: Record<SessionTileId, () => void> = {
    ask: () => void run("ask", async () => void (await askForUpdate(draft, session.id))),
    land: () =>
      void run("land", async () => {
        if (await sent(on ? LAND_OFF_TEXT : LAND_ON_TEXT)) setAutoLand(session.id, !on);
      }),
    compact: () => {
      if (compact?.how === "terminal") draft.command?.(session.id, `/${COMPACT}`);
      else void run("compact", async () => void (await sent(`/${COMPACT}`)));
    },
    report: () => setReporting((open) => !open),
    map: () => setView("map"),
    release: () => setReleasing(true),
  };
  const release = async (to: Session | undefined): Promise<boolean> => {
    const before = session;
    const answer = await draft.releaseHolds?.(session.id, to?.id);
    if (answer === undefined) return false;
    setReleasing(false);
    setReleased({ answer, to, before });
    return true;
  };
  const report = async (words: string): Promise<boolean> => {
    if (repository === undefined) return false;
    const id = started.current ?? (await draft.start(undefined, startIn ?? repository.root));
    if (id === undefined) return false;
    started.current = id;
    const brief = reportBrief(words, session, { at: new Date(), screen: "Cockpit, flight controls", root: repository.root, ...(repository.name === undefined ? {} : { repository: repository.name }), events, ...(refused === undefined ? {} : { refused }) });
    if ((await draft.send(id, plain(brief))) === false) return false;
    started.current = undefined;
    select({ kind: "session", key: `session:${id}` });
    return true;
  };
  const tip: Record<SessionTileId, string> = {
    ask: "Ask the Session for an update",
    land: on ? "Turn auto land off" : "Turn auto land on",
    compact: "Compact the Session's context",
    report: "Report an issue with the Session",
    map: "Map",
    release: "Release or hand over everything the Session holds",
  };
  const dialog = releasing ? (
    <ReleaseHoldsDialog session={session} held={heldOf(session)} targets={handTargets(session, draft.get())} send={release} onCancel={() => setReleasing(false)} />
  ) : null;
  if (released !== undefined) {
    return <HoldsReleasedBox released={released.answer} to={released.to} nameOfRow={(row) => nameOfHeldRow(released.before, row)} onClose={() => setReleased(undefined)} />;
  }
  if (reporting) {
    return (
      <ReportBox
        keep={`report:${session.id}`}
        send={report}
        onClose={() => {
          started.current = undefined;
          setReporting(false);
        }}
      />
    );
  }
  return (
    <>
      {dialog}
      <div className="deck-controls__block" data-subject="session">
        {SESSION_TILES.map((tile, at) => {
          const Glyph = GLYPH[tile.id];
          const why = off[tile.id];
          return (
            <Tooltip key={tile.id} label={why ?? tip[tile.id]} shortcut={String(at + 1)} asChild>
              <button
                type="button"
                className="deck-tile"
                aria-label={tile.label}
                aria-pressed={tile.id === "map" || tile.id === "release" ? false : pending === tile.id || (tile.id === "land" && on)}
                aria-busy={pending === tile.id || undefined}
                aria-disabled={why !== undefined || undefined}
                onClick={() => why === undefined && press[tile.id]()}
              >
                <Glyph size={20} aria-hidden="true" />
                <span className="deck-tile__text">{tile.text}</span>
                <Kbd className="deck-tile__key" aria-hidden="true">
                  {at + 1}
                </Kbd>
              </button>
            </Tooltip>
          );
        })}
      </div>
    </>
  );
}
