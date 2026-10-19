// Where a repository keeps its Armada configuration, drawn out of the way of the bands above the message box: a labelled chip on the first Job, a bare glyph on
// every Job after, each opening the menu of the four places. Or the first Job's card at the top of its thread, which lists the four. Nothing blocks the box.
// **Mock only**: Fleet keeps none of it yet.

import { useEffect, useRef, useState } from "react";
import { Button, Card, CardTitle, PLACE_GLYPH, RunsPlaces, Tooltip, placePhrase, type RunsPlace, type SyncedFolder } from "@armada/components";

import "./runs-place-control.css";

type Where = {
  place: RunsPlace;
  synced: readonly SyncedFolder[];
  profiles: readonly string[];
  onPick: (place: RunsPlace) => void;
  /** The menu is put away, by a pick or by Esc: the message box takes the caret back. */
  onDone: () => void;
};

const said = (place: RunsPlace) => `Armada configuration kept ${placePhrase(place)}`;
const named = (place: RunsPlace) => placePhrase(place).replace(/^./, (first) => first.toUpperCase());

/**
 * The chip or the glyph, and the menu it opens. `labelled` is the first Job's chip, `rises` opens the menu upward from a control that sits low on the page.
 * J and K move in the menu, Enter picks, Esc puts it away.
 */
export function RunsPlaceControl({ place, synced, profiles, onPick, onDone, labelled, rises }: Where & { labelled: boolean; rises: boolean }) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLSpanElement>(null);
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
    <span ref={root} className="armada-place-control">
      {labelled ? (
        <Tooltip label="Change where Armada configuration is kept">
          <button type="button" className="armada-place-control__chip" aria-expanded={open} aria-haspopup="true" onClick={() => setOpen(!open)}>
            <Glyph size={12} strokeWidth={2} aria-hidden />
            <span>{`Armada configuration: ${named(place)}`}</span>
          </button>
        </Tooltip>
      ) : (
        <Tooltip label={said(place)}>
          <Button type="button" variant="ghost" size="sm" iconOnly aria-label={said(place)} aria-expanded={open} aria-haspopup="true" onClick={() => setOpen(!open)}>
            <Glyph size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      )}
      {!open ? null : (
        <div className="armada-place-control__menu" data-rises={rises || undefined}>
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
    </span>
  );
}

/** The first Job's card at the top of its thread: the four places in place, the one it stands on marked. A pick changes it and the box takes the caret back. */
export function RunsPlaceCard({ place, synced, profiles, onPick, onDone }: Where) {
  return (
    <Card className="armada-place-card">
      <CardTitle className="armada-place-card__title">Armada configuration</CardTitle>
      <RunsPlaces seize={false} synced={synced} profiles={profiles} current={place} onPick={(next) => { onPick(next); onDone(); }} onClose={onDone} />
    </Card>
  );
}
