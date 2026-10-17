// The Pause act on a Session's row in the Work list, and its paused mark. Mounted once in the row's item; nothing else in the row knows of it.
// The act shows on hover and on focus, Resume while the Session is paused, and does what the tile on Flight control does.

import { CirclePause, Play } from "lucide-react";
import { Button, PausedMark, Tooltip } from "@armada/components";

import { useSessions, useSessionsDraft } from "../../sessions-draft";
import { PAUSE_KEY } from "./session-tiles";

export function RowPause({ id }: { id: string }) {
  const draft = useSessionsDraft();
  const paused = useSessions().find((one) => one.id === id)?.paused === true;
  const act = paused ? draft?.resume : draft?.pause;
  if (act === undefined) return null;
  const label = paused ? "Resume" : "Pause";
  const Glyph = paused ? Play : CirclePause;
  return (
    <span className="deck-row-pause" data-paused={paused || undefined}>
      {paused ? <PausedMark said="Paused" /> : null}
      <Tooltip label={label} shortcut={PAUSE_KEY} asChild>
        <Button variant="ghost" size="sm" iconOnly className="deck-row-pause__act" data-act="row-pause" aria-label={label} onClick={() => void act(id)}>
          <Glyph size={14} aria-hidden="true" />
        </Button>
      </Tooltip>
    </span>
  );
}
