// Where a repository keeps its Armada configuration, drawn out of the way of the bands above the message box: a labelled chip on the first Job, a bare glyph on
// every Job after, each opening the menu of the four places. Or the first Job's card at the top of its thread, which lists the four. Nothing blocks the box.
// **Mock only**: Fleet keeps none of it yet.

import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties } from "react";
import { createPortal } from "react-dom";
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

/** The space kept between the menu and the control, and the window's edge. */
const MARGIN = 8;

const said = (place: RunsPlace) => `Armada configuration kept ${placePhrase(place)}`;
const named = (place: RunsPlace) => placePhrase(place).replace(/^./, (first) => first.toUpperCase());

/**
 * The chip or the glyph, and the menu it opens. `labelled` is the first Job's chip: `short` where the row is crowded, which is a header's, and `full`
 * where it is not. `rises` opens the menu upward from a control that sits low on the page.
 * J and K move in the menu, Enter picks, Esc puts it away.
 */
export function RunsPlaceControl({ place, synced, profiles, onPick, onDone, labelled, rises }: Where & { labelled: false | "short" | "full"; rises: boolean }) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLSpanElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  const [at, setAt] = useState<CSSProperties>({});
  const Glyph = PLACE_GLYPH[place.id];

  // The menu is laid on the page, not in the header or the toolbar, whose card clips what runs past it and scrolls to the focus inside it.
  useLayoutEffect(() => {
    const box = root.current?.getBoundingClientRect();
    if (!open || box === undefined) return;
    const width = menu.current?.offsetWidth ?? 0;
    const left = Math.max(MARGIN, Math.min(box.left, window.innerWidth - width - MARGIN));
    setAt(rises ? { left, bottom: window.innerHeight - box.top + MARGIN } : { left, top: box.bottom + MARGIN });
  }, [open, rises]);

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
    onDone();
  };

  return (
    <span ref={root} className="armada-place-control">
      {labelled ? (
        <Tooltip label={labelled === "short" ? said(place) : "Change where Armada configuration is kept"}>
          <button type="button" className="armada-place-control__chip" aria-expanded={open} aria-haspopup="true" {...(labelled === "short" ? { "aria-label": said(place) } : {})} onClick={() => setOpen(!open)}>
            <Glyph size={12} strokeWidth={2} aria-hidden />
            <span>{labelled === "short" ? `Kept ${placePhrase(place)}` : `Armada configuration: ${named(place)}`}</span>
          </button>
        </Tooltip>
      ) : (
        <Tooltip label={said(place)}>
          <Button type="button" variant="ghost" size="sm" iconOnly aria-label={said(place)} aria-expanded={open} aria-haspopup="true" onClick={() => setOpen(!open)}>
            <Glyph size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      )}
      {!open ? null : createPortal(
        <div ref={menu} className="armada-place-control__menu" style={at}>
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
        </div>,
        document.body,
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
