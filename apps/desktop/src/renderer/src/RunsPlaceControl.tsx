// Where a repository keeps its Armada configuration, drawn out of the way of the bands above the message box: the first Job's card at the top of its thread,
// which lists the four places, and a bare glyph in the header of every Job after, which opens the menu of them. Nothing blocks the box.
// A place Fleet refuses stays unchosen, and its words show under the places.

import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties } from "react";
import { createPortal } from "react-dom";
import { Button, Card, CardTitle, PLACE_GLYPH, RunsPlaces, Tooltip, placePhrase, type RunsPlace, type SyncedFolder } from "@armada/components";

import "./runs-place-control.css";

type Where = {
  place: RunsPlace;
  synced: readonly SyncedFolder[];
  profiles: readonly string[];
  /** Asks Fleet to keep the runs there. Answers the words of a refusal, or nothing where it did. */
  onPick: (place: RunsPlace) => Promise<string | undefined>;
  /** Asks for a folder on this Mac, for a synced place. */
  onFolder: () => Promise<string | null>;
  /** The menu is put away, by a pick or by Esc: the message box takes the caret back. */
  onDone: () => void;
};

/** The space kept between the menu and the control, and the window's edge. */
const MARGIN = 8;

/** What the places are for, said once above them. */
const LINE = "Where Armada keeps this repository's configuration: how its tests, build and dev server run.";
const said = (place: RunsPlace) => `Armada configuration kept ${placePhrase(place)}`;

/** The glyph, and the menu it opens. J and K move in the menu, Enter picks, Esc puts it away. */
export function RunsPlaceControl({ place, synced, profiles, onPick, onFolder, onDone }: Where) {
  const [open, setOpen] = useState(false);
  const [refused, setRefused] = useState<string | undefined>();
  const root = useRef<HTMLSpanElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  const [at, setAt] = useState<CSSProperties>({});
  const Glyph = PLACE_GLYPH[place.id];

  // The menu is laid on the page, not in the header, whose card clips what runs past it and scrolls to the focus inside it.
  useLayoutEffect(() => {
    const box = root.current?.getBoundingClientRect();
    if (!open || box === undefined) return;
    const width = menu.current?.offsetWidth ?? 0;
    const left = Math.max(MARGIN, Math.min(box.left, window.innerWidth - width - MARGIN));
    setAt({ left, top: box.bottom + MARGIN });
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const away = (event: MouseEvent) => {
      const target = event.target as Node;
      if (!root.current?.contains(target) && !menu.current?.contains(target)) setOpen(false);
    };
    window.addEventListener("mousedown", away);
    return () => window.removeEventListener("mousedown", away);
  }, [open]);

  const close = () => {
    setOpen(false);
    setRefused(undefined);
    onDone();
  };

  return (
    <span ref={root} className="armada-place-control">
      <Tooltip label={said(place)}>
        <Button type="button" variant="ghost" size="sm" iconOnly aria-label={said(place)} aria-expanded={open} aria-haspopup="true" onClick={() => setOpen(!open)}>
          <Glyph size={16} strokeWidth={2} aria-hidden />
        </Button>
      </Tooltip>
      {!open ? null : createPortal(
        <div ref={menu} className="armada-place-control__menu" style={at}>
          <p className="armada-place-card__line armada-place-control__line">{LINE}</p>
          <RunsPlaces
            synced={synced}
            profiles={profiles}
            current={place}
            refused={refused}
            onFolder={onFolder}
            onPick={(next) => void onPick(next).then((said) => (said === undefined ? close() : setRefused(said)))}
            onClose={close}
          />
        </div>,
        document.body,
      )}
    </span>
  );
}

/** The first Job's card at the top of its thread: the four places in place, the one it stands on marked. A pick changes it and the box takes the caret back. */
export function RunsPlaceCard({ place, synced, profiles, onPick, onFolder, onDone }: Where) {
  const [refused, setRefused] = useState<string | undefined>();
  return (
    <Card className="armada-place-card">
      <CardTitle className="armada-place-card__title">Armada configuration</CardTitle>
      <p className="armada-place-card__line">{LINE}</p>
      <RunsPlaces
        seize={false}
        synced={synced}
        profiles={profiles}
        current={place}
        refused={refused}
        onFolder={onFolder}
        onPick={(next) => void onPick(next).then((said) => (said === undefined ? (setRefused(undefined), onDone()) : setRefused(said)))}
        onClose={onDone}
      />
    </Card>
  );
}
