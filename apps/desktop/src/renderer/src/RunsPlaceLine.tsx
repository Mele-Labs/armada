// Where a new Job's repository keeps its runs, as one line above the message box: the place it stands on, and a menu of the four. Nothing blocks the box;
// a Job sent without touching it keeps the place shown. The line is the Job's own until its first message.

import { useEffect, useRef, useState } from "react";
import { ChevronUp } from "lucide-react";
import { PLACE_GLYPH, RunsPlaces, Tooltip, placePhrase, type RunsPlace, type SyncedFolder } from "@armada/components";

import "./runs-place-line.css";

export function RunsPlaceLine({ place, synced, profiles, onPick, onDone }: {
  place: RunsPlace;
  synced: readonly SyncedFolder[];
  profiles: readonly string[];
  onPick: (place: RunsPlace) => void;
  /** The menu is put away, by a pick or by Esc: the message box takes the caret back. */
  onDone: () => void;
}) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const Glyph = PLACE_GLYPH[place.id];

  useEffect(() => {
    if (!open) return;
    const away = (event: MouseEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    window.addEventListener("mousedown", away);
    return () => window.removeEventListener("mousedown", away);
  }, [open]);

  const close = () => {
    setOpen(false);
    onDone();
  };

  return (
    <div ref={root} className="armada-place-line">
      <Tooltip label="Change where runs are kept">
        <button type="button" className="armada-place-line__button" aria-expanded={open} aria-haspopup="true" onClick={() => setOpen(!open)}>
          <Glyph size={12} strokeWidth={2} aria-hidden />
          <span>{`Runs kept ${placePhrase(place)}`}</span>
          <ChevronUp size={12} strokeWidth={2} aria-hidden />
        </button>
      </Tooltip>
      {!open ? null : (
        <div className="armada-place-line__menu">
          <RunsPlaces
            synced={synced}
            profiles={profiles}
            current={place}
            onPick={(next) => {
              onPick(next);
              close();
            }}
            onClose={close}
          />
        </div>
      )}
    </div>
  );
}
