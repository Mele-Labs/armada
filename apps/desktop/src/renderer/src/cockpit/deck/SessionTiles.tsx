// The flight controls' tiles while a Session is open on the stage: the same block, size and keys, `1` to `5`, as the six usual
// ones, with the Session's own acts. What each does and why one is off is in `session-tiles.ts`. A refusal from Fleet is the
// toast the Session's composer shows, since the same `draft.send` carries it. The owner's ask of 15 Oct 2026.

import { useState } from "react";
import { Bug, FoldVertical, MessageCircleMore, Orbit, PlaneLanding } from "lucide-react";
import { Kbd, Tooltip } from "@armada/components";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { setAutoLand, useAutoLand } from "./auto-land";
import { askForUpdate } from "./acts";
import { COMPACT, LAND_OFF_TEXT, LAND_ON_TEXT, SESSION_TILES, offeredOf, plain, type SessionTileId } from "./session-tiles";
import { setView } from "./store";

const GLYPH = { ask: MessageCircleMore, land: PlaneLanding, compact: FoldVertical, report: Bug, map: Orbit } as const;

export function SessionTiles({ session, draft }: { session: Session; draft: SessionsDraft }) {
  const [pending, setPending] = useState<SessionTileId | undefined>();
  const on = useAutoLand(session.id);
  const { off, compact } = offeredOf(session, draft);
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
    report: () => {},
    map: () => setView("map"),
  };
  const tip: Record<SessionTileId, string> = {
    ask: "Ask the Session for an update",
    land: on ? "Turn auto land off" : "Turn auto land on",
    compact: "Compact the Session's context",
    report: "Report an issue with the Session",
    map: "Map",
  };
  return (
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
              aria-pressed={tile.id === "map" ? false : pending === tile.id || (tile.id === "land" && on)}
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
  );
}
