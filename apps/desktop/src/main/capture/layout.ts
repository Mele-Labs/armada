// Where the walk window's three views sit, and which panel it has. Pure, so it is read without a window.
// `docs/practices/capture-window.md`, *The panel beside the page*.

/** A pull request's walk, opened from the Reviews tab: **no Session shows it**, so a note goes to the Session that holds the pull request, or stays in Armada. */
export type ReviewWalkOwner = {
  walkId: string;
  manifestId: string;
  number: number;
  title: string;
  /** The Session of his that holds the pull request, where one does. */
  holder?: { id: string; title: string };
  /** Where the change runs, for the note's words: the page the walk is on may be Bridge's own runner. */
  address: string;
};

/** Where a note made in the window lands: the Run's Studio, the Session that showed the page, or a pull request's holder. */
export type LandsOn =
  | { studio: { id: string; name: string | null } }
  | { session: { id: string; title: string } }
  | { review: ReviewWalkOwner };

/** What the panel beside the page holds: the Session that opened the window. */
export type PanelSubject = { kind: "session"; id: string };

export type Rect = { x: number; y: number; width: number; height: number };

/** A Session's window holds its Session. **A Studio's has none**: it is read on Bridge's window. */
export function panelFor(landsOn: LandsOn): PanelSubject | null {
  if ("session" in landsOn) return { kind: "session", id: landsOn.session.id };
  return null;
}

/** The query the panel's document is loaded with. **Its only input from main beside what every Bridge window reads.** */
export function panelQuery(subject: PanelSubject): Record<string, string> {
  return { [subject.kind]: subject.id };
}

/**
 * How many rows the bar draws. **A queued bar's takes sit in its first row**, beside the walks, so it has a second row
 * only for the walk's steps or a status, pending notes included; any other bar gives the takes and the walk a row each,
 * and a status one more, a page loading included. **Only a queued bar says notes are pending**, so `held` takes a row there alone.
 */
export function barRows(bar: { queued: boolean; takes: boolean; walk: boolean; said: boolean; held?: boolean; loading?: boolean }): number {
  const status = bar.said || bar.loading === true;
  if (bar.queued) return 1 + (bar.walk || status || bar.held === true ? 1 : 0);
  return 1 + (bar.takes ? 1 : 0) + (bar.walk ? 1 : 0) + (status ? 1 : 0);
}

/**
 * The window's content size, the bar's strip, whether the bar covers the page, how far down an open menu or tooltip of
 * the bar's draws (0 or absent: none), and the panel's width (0: none, or closed).
 */
export type WalkFrame = { width: number; height: number; strip: number; covered: boolean; hang?: number; panel: number };

/**
 * The page and the bar share the column left of the panel, and the panel takes the right edge at full
 * height. **Covering the page covers the column alone**, so the thread stays answerable under armed
 * capture or a question drawn over the page. **An open menu or tooltip of the bar's covers it as far down as it draws**,
 * and no further than the frame. A narrow window gives way from the panel first, so the
 * page keeps `floor`.
 */
export function walkLayout(frame: WalkFrame, floor: number): { page: Rect; bar: Rect; panel: Rect } {
  const panel = Math.max(0, Math.min(frame.panel, frame.width - floor));
  const column = frame.width - panel;
  return {
    page: { x: 0, y: frame.strip, width: column, height: Math.max(0, frame.height - frame.strip) },
    bar: { x: 0, y: 0, width: column, height: frame.covered ? frame.height : Math.min(frame.height, Math.max(frame.strip, Math.ceil(frame.hang ?? 0))) },
    panel: { x: column, y: 0, width: panel, height: panel === 0 ? 0 : frame.height },
  };
}
