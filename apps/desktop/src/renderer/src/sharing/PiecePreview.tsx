// One piece of a package as Import draws it: a card under a band naming its kind in its hue, the repository it lands on as a chip,
// and where it stands as a glyph with a tooltip, over the piece drawn the way Bridge will draw it once added. A tile is a flight tile
// with its command under it, a review tool the review's own tile, a workflow its row on the Workflows page, and an armada.yml the
// Manifest's mark. The owner's note on the walk `sharing-in-bridge`, 21 Oct 2026.

import type { ReactNode } from "react";
import { Briefcase, CircleCheck, CircleDashed, ClipboardList, FileCog, FolderGit2, RefreshCw, Workflow, Zap } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { Tooltip } from "@armada/components";
import type { FlightControlKind, PieceKind } from "@armada/protocol";

import { KIND_GLYPH } from "../cockpit/deck/flight-controls";
import "../cockpit/deck/deck.css";
import { glyphOf } from "../cockpit/deck/flight-glyphs";

const KIND: Record<PieceKind, { said: string; Glyph: LucideIcon }> = {
  tile: { said: "Tile", Glyph: Zap },
  workflow: { said: "Workflow", Glyph: Workflow },
  review_tool: { said: "Review tool", Glyph: ClipboardList },
  manifest: { said: "armada.yml", Glyph: FileCog },
};

/** Where a piece stands: about to be added, about to replace one of yours, added, or waiting for a repository. */
export type PieceState = "new" | "replaces" | "added" | "replaced" | "waiting";

const STATE: Record<Exclude<PieceState, "new">, { said: string; Glyph: LucideIcon }> = {
  replaces: { said: "Replaces yours", Glyph: RefreshCw },
  added: { said: "Added", Glyph: CircleCheck },
  replaced: { said: "Replaced", Glyph: CircleCheck },
  waiting: { said: "Waiting for a repository", Glyph: CircleDashed },
};

export type Previewed = { kind: PieceKind; id: string; name: string; action?: FlightControlKind | undefined; commands?: readonly string[] | undefined };

/** The piece as Bridge will draw it, never pressable here. */
function Preview({ piece }: { piece: Previewed }) {
  if (piece.kind === "tile") {
    const Glyph = glyphOf(piece.action === undefined ? undefined : KIND_GLYPH[piece.action], "zap");
    return (
      <span className="deck-tile import-preview__tile" aria-hidden="true">
        <Glyph size={20} />
        <span className="deck-tile__text">{piece.name}</span>
      </span>
    );
  }
  if (piece.kind === "review_tool") {
    return (
      <span className="deck-tile import-preview__tile import-preview__tool" aria-hidden="true">
        <ClipboardList size={16} />
        <span className="deck-tile__text">{piece.name}</span>
        <span className="import-preview__source">
          <Briefcase size={12} strokeWidth={2} />
        </span>
      </span>
    );
  }
  if (piece.kind === "workflow") {
    return (
      <span className="import-preview__row" aria-hidden="true">
        <Briefcase size={12} strokeWidth={2} />
        <span className="import-preview__id">{piece.id}</span>
      </span>
    );
  }
  return (
    <span className="import-preview__row" aria-hidden="true">
      <FileCog size={12} strokeWidth={2} />
      <span className="import-preview__id">armada.yml</span>
    </span>
  );
}

/** One piece, as a card. `children` is drawn under the preview: where to choose a repository, for one waiting. */
export function PieceCard({ piece, where, state, children }: { piece: Previewed; where: string; state: PieceState; children?: ReactNode }) {
  const kind = KIND[piece.kind];
  const stands = state === "new" ? undefined : STATE[state];
  return (
    <li className="import-card" data-kind={piece.kind} data-state={state} aria-label={`${kind.said} ${piece.name}`}>
      <span className="import-card__band">
        <span className="import-card__kind">
          <kind.Glyph size={12} strokeWidth={2} aria-hidden="true" />
          {kind.said}
        </span>
        <span className="import-card__chip">
          <FolderGit2 size={12} strokeWidth={2} aria-hidden="true" />
          {where}
        </span>
        {stands === undefined ? null : (
          <Tooltip label={stands.said}>
            <span className="import-card__state" role="img" aria-label={stands.said}>
              <stands.Glyph size={16} strokeWidth={2} aria-hidden="true" />
            </span>
          </Tooltip>
        )}
      </span>
      <span className="import-card__body">
        <Preview piece={piece} />
        {(piece.commands ?? []).length === 0 ? null : (
          <span className="import-card__commands">
            {(piece.commands ?? []).map((command) => (
              <code key={command} className="import-card__command" aria-label="Command">
                {command}
              </code>
            ))}
          </span>
        )}
      </span>
      {children}
    </li>
  );
}
