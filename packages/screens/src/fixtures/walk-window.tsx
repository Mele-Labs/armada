// What stands in for Bridge's own window on a Job's server, in a browser that
// has no second window to open: the capture window's bar, with no Studio, over
// the page it would load. `prototype-fleet.ts` opens it. Dev-only.
//
// **A window, so the app beside it stays usable**: dragged by its strip and
// resized from its corner, and nothing under it is blocked — the real one is
// a window of its own, and the owner moves between Jobs with it open.
//
// **Capture lands on the Job**, as main's window does since protocol 23.18:
// Capture or ⌥⌘A arms it, a press inside the page picks what is under it, and
// the note is handed to `onNote`. The page is this mock on the same origin, so
// the frame's document is reachable — main asks its page through a script.
//
// **One window for every walk.** `queueWalk` adds a walk to the window's queue, oldest shown first,
// and opens the window only when none is open: a walk that arrives while it is open joins the row on
// the bar and never takes the page. Approve moves to the next walk and the last one closes the window.
// The panel beside the page is a host element this file draws and `useWalkQueue` publishes: Bridge's
// own Job view is drawn into it from inside the app, where its providers are (`mock/WalkJobPanel.tsx`).

import { StrictMode, useEffect, useRef, useState, useSyncExternalStore, type ReactNode, type RefObject } from "react";
import { createRoot, type Root } from "react-dom/client";
import { Button, CaptureBar, CaptureQueueList, WindowQuestion, variationLetter, type CaptureBarApproval, type CaptureBarLayout, type CaptureBarQueue, type CaptureBarReadiness, type CaptureBarWalk } from "@armada/components";

/** Where the walk a page is playing stands, as its bar draws it. `at` counts from 1. */
export type WalkState = { name: string; at: number; of: number; said: string; autoplay: boolean; ready: boolean; done: boolean };

/**
 * **What the walk in a page offers the window that frames it**, on the page's `window.__armadaWalk`.
 * The real window would reach it by main running script in the page, as it does for capture and for
 * a question; here the frame is the same origin and the stand-in reads it straight off.
 */
export type WalkHandle = {
  state: () => WalkState;
  /** Told after every change in `state`. Returns the way to stop hearing. */
  subscribe: (hear: () => void) => () => void;
  next: () => void;
  /** Replays the page to the step before. */
  back: () => void;
  autoplay: (on: boolean) => void;
  /** Replays the page from step 1. */
  replay: () => void;
  /** The page draws its own card, or leaves it to the bar. The ring stays either way. */
  hostCard: (hidden: boolean) => void;
};

declare global {
  interface Window {
    __armadaWalk?: WalkHandle;
  }
}

/** What a press inside the page picked, as a walk note describes it. */
export type Picked = { element: string; selector: string; location: string; variation?: Taken };

/** One take of a change, a page of its own. */
export type Variation = { name: string; url: string };

/** The take on screen, as a note and an Approve name it: `B, Cards`. */
export type Taken = { letter: string; name: string };

export type WalkWindowOptions = {
  /** What a note goes to, as the bar names it: a Job's handle, or a Session's title. */
  job: string;
  /** Which of the two it is, as the note's Send says. Absent is a Job. */
  into?: "Job" | "Session";
  /** What the bar's name says: the window's own title. Absent is the `run`; `null` is a window titled by its address. */
  title?: string | null;
  /** What the bar names as the page's address, where the page in the frame stands in for another. Absent: the page's own. */
  address?: string;
  /** The press on the owner's name, after Bridge comes forward. Absent: Bridge comes forward on what it shows. */
  onOpenOwner?: () => void;
  onNote: (said: string, picked: Picked) => void;
  /** Approve, pressed on the page at `address`: what the Job or the Session is told. */
  onApprove?: (address: string, variation?: Taken) => Promise<{ ok: true } | { ok: false; said: string }>;
  /** The takes shown, the first on screen. **Absent: the one page at `url`.** */
  variations?: readonly Variation[];
  /** Ready or still changing, as the opener says when it opens it. `markWalkWindow` moves it after. */
  readiness?: CaptureBarReadiness;
  /** The window was closed, by its own Close or by another opening in its place. */
  onClose?: () => void;
};

/** What a walk in the queue adds to a window's options. */
export type QueuedWalkOptions = WalkWindowOptions & {
  /** What its entry on the bar reads as: the Job's own name, or a pull request's title with its number. */
  entry: { name: string; pull?: number };
  /** When the walk was shown. The queue runs oldest first. */
  since: string;
  /** The Job whose view the panel beside the page draws while this walk is on screen. Absent: no panel. */
  panelFor?: string;
  /** What the Job asked about this page when it was shown. */
  asking?: string;
  /** The words sent in answer to a question about this page. */
  onAnswer?: (said: string) => void;
};

/**
 * **Where the walks are named is the take**, and `?take=` on the page's address says which: `tabs` (A), `switcher` (B) or
 * `panel` (C). Absent or unknown is `tabs`. Every take draws at most two rows.
 */
export function takeOfPage(): CaptureBarLayout {
  if (typeof window === "undefined") return "tabs";
  const asked = new URLSearchParams(window.location.search).get("take");
  return asked === "switcher" || asked === "panel" ? asked : "tabs";
}

/** A walk waiting in the window, or on screen. `asking` is what its Job asked while the walk was not on screen. */
export type QueuedWalk = { id: string; run: string; url: string; options: QueuedWalkOptions; asking?: string };

/** The queue as the window and the app beside it read it: the walks, the one on screen, and the element the panel is drawn into. */
export type WalkQueue = { walks: readonly QueuedWalk[]; at: string; panelHost: HTMLElement | null };

let held: { root: Root; host: HTMLElement; onClose?: () => void } | null = null;
let mark: ((readiness: CaptureBarReadiness) => void) | null = null;
let queue: WalkQueue | null = null;
const hearing = new Set<() => void>();

function setQueue(next: WalkQueue | null): void {
  queue = next;
  hearing.forEach((hear) => hear());
}

/** The window's queue, or null where no queued window is open. */
export function useWalkQueue(): WalkQueue | null {
  return useSyncExternalStore(
    (hear) => {
      hearing.add(hear);
      return () => void hearing.delete(hear);
    },
    () => queue,
  );
}

/** The walk on screen, if the window is a queued one. */
export const walkOnScreen = (of: WalkQueue | null): QueuedWalk | undefined => of?.walks.find((one) => one.id === of.at);

/**
 * A walk joins the window. **Opens it only when none is open**; otherwise the walk goes in its place by
 * `since` and the page stays, and a walk already queued under `id` is updated where it stands.
 */
export function queueWalk(id: string, run: string, url: string, options: QueuedWalkOptions): void {
  const walk: QueuedWalk = { id, run, url, options, ...(options.asking === undefined ? {} : { asking: options.asking }) };
  if (queue === null) {
    closeWalkWindow();
    mountWindow((host) => <QueuedWalkWindow host={host} />);
    setQueue({ walks: [walk], at: id, panelHost: null });
    return;
  }
  const standing = queue.walks.find((one) => one.id === id);
  if (standing !== undefined) {
    setQueue({ ...queue, walks: queue.walks.map((one) => (one.id === id ? { ...walk, ...(one.asking === undefined ? {} : { asking: one.asking }) } : one)) });
    return;
  }
  const after = queue.walks.findIndex((one) => Date.parse(one.options.since) > Date.parse(options.since));
  const walks = after < 0 ? [...queue.walks, walk] : [...queue.walks.slice(0, after), walk, ...queue.walks.slice(after)];
  setQueue({ ...queue, walks });
}

/** Press on an entry, or ⌥⌘ and an arrow: the page changes to that walk. */
export function jumpToWalk(id: string): void {
  if (queue === null || queue.at === id || !queue.walks.some((one) => one.id === id)) return;
  setQueue({ ...queue, at: id, panelHost: null });
}

/**
 * A Job asks about a page. **An entry that is not on screen takes the mark and the page stays**; the question
 * is `asking`, or nothing to take it back.
 */
export function askFromWalk(id: string, asking: string | undefined): void {
  if (queue === null) return;
  setQueue({ ...queue, walks: queue.walks.map((one) => (one.id !== id ? one : asking === undefined ? withoutAsking(one) : { ...one, asking })) });
}

const withoutAsking = ({ asking, ...rest }: QueuedWalk): QueuedWalk => {
  void asking;
  return rest;
};

/** The walk is done: it leaves the queue and the next comes on screen. The last one closes the window. */
function settleWalk(id: string): void {
  if (queue === null) return;
  const at = queue.walks.findIndex((one) => one.id === id);
  const rest = queue.walks.filter((one) => one.id !== id);
  const done = queue.walks[at];
  if (rest.length === 0) {
    closeWalkWindow();
    return;
  }
  // The one that followed it, or the first again when it was the last.
  const next = rest[at] ?? rest[0]!;
  setQueue({ walks: rest, at: queue.at === id ? next.id : queue.at, panelHost: queue.at === id ? null : queue.panelHost });
  done?.options.onClose?.();
}

/** Where the panel beside the page is drawn. Set by the walk on screen as it mounts, and put away as it goes. */
function publishPanelHost(element: HTMLElement | null): void {
  if (queue !== null && queue.panelHost !== element) setQueue({ ...queue, panelHost: element });
}

/** Bridge's window for a queued walk: the frame, and the walk on screen in it. */
function QueuedWalkWindow({ host }: { host: HTMLElement }) {
  const now = useWalkQueue();
  const walk = walkOnScreen(now);
  if (now === null || walk === undefined) return null;
  const { options } = walk;
  // What was asked is the queue's to hold from here, so a question that was answered stays answered.
  const { asking: _asked, ...shown } = options;
  void _asked;
  const layout = takeOfPage();
  const at = now.walks.findIndex((one) => one.id === walk.id);
  const row: CaptureBarQueue = {
    entries: now.walks.map((one) => ({
      id: one.id,
      name: one.options.entry.name,
      ...(one.options.entry.pull === undefined ? {} : { pull: one.options.entry.pull }),
      title: one.options.title ?? one.run,
      address: one.options.address ?? new URL(one.url).origin,
      aim: `Notes go to ${one.options.job}`,
      ...(one.options.readiness === undefined ? {} : { readiness: one.options.readiness }),
      ...(one.asking === undefined ? {} : { asking: one.asking }),
    })),
    at,
    onPick: (to) => jumpToWalk(now.walks[to]!.id),
    onStep: (by) => jumpToWalk(now.walks[(at + by + now.walks.length) % now.walks.length]!.id),
  };
  return (
    <WalkFrame run={walk.run} host={host} wide={layout === "panel" || options.panelFor !== undefined}>
      <WalkEntry
        key={walk.id}
        run={walk.run}
        url={walk.url}
        host={host}
        {...shown}
        queue={row}
        layout={layout}
        {...(walk.asking === undefined ? {} : { asking: walk.asking })}
        onAsked={(said) => {
          options.onAnswer?.(said);
          askFromWalk(walk.id, undefined);
        }}
        onSettled={() => settleWalk(walk.id)}
      />
    </WalkFrame>
  );
}

/** The opener says the page is ready, or that it is changing it again. Nothing where no window is open. */
export function markWalkWindow(readiness: CaptureBarReadiness): void {
  mark?.(readiness);
}

/** The mock itself on another scenario: the one address a browser page can serve as a page to show. */
export function mockPage(): string {
  // The roster is also read in node, by `scenario.test.ts`, where there is no page.
  if (typeof window === "undefined") return "http://localhost:41311/?scenario=default&walked";
  const url = new URL(window.location.href);
  url.search = "?scenario=default&walked";
  return url.toString();
}

/** Close what is open. */
export function closeWalkWindow(): void {
  document.documentElement.removeAttribute("data-walking");
  const closing = held;
  held = null;
  const waiting = queue?.walks ?? [];
  if (queue !== null) setQueue(null);
  closing?.root.unmount();
  closing?.host.remove();
  closing?.onClose?.();
  waiting.forEach((one) => one.options.onClose?.());
}

/** Put the window's host on the page and draw into it. */
function mountWindow(draw: (host: HTMLElement) => ReactNode, onClose?: () => void): void {
  const host = document.createElement("div");
  host.className = "armada-mock-walk-window";
  document.body.append(host);
  const root = createRoot(host);
  root.render(<StrictMode>{draw(host)}</StrictMode>);
  held = { root, host, ...(onClose === undefined ? {} : { onClose }) };
}

/**
 * Open the stand-in on `url`, replacing one already open — main raises the
 * one window per server, and a page has nothing to raise.
 *
 * **No page under test.** A frame of the tester's own page would start the
 * suite again inside itself, so a test sees the bar and an empty frame.
 */
export function openWalkWindow(run: string, url: string, options: WalkWindowOptions): void {
  closeWalkWindow();
  mountWindow(
    (host) => (
      <WalkFrame run={run} host={host}>
        <WalkEntry run={run} url={url} host={host} {...options} />
      </WalkFrame>
    ),
    options.onClose,
  );
}

/** ⌥⌘A or ⌥⌘C, the two chords main's window takes before its page does. */
const isChord = (event: KeyboardEvent): boolean =>
  (event.code === "KeyA" || event.code === "KeyC") && event.metaKey && event.altKey && !event.ctrlKey && !event.shiftKey;

/**
 * The chord, claimed while a stand-in is open. **Listened for at import**, so
 * it runs ahead of the dev annotation layer `main.tsx` mounts after the app:
 * a real walk window is a window of its own and Bridge's layer never sees its
 * keys, but here both share one page and the layer would arm as well.
 */
let claimChord: (() => void) | null = null;
if (typeof window !== "undefined") {
  window.addEventListener(
    "keydown",
    (event) => {
      if (claimChord === null || !isChord(event)) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      claimChord();
    },
    true,
  );
}

/**
 * The window itself, which stays put as the walk in it changes: its place and size are the person's, and
 * a walk coming on screen draws inside it. `wide` makes room for the panel beside the page.
 */
function WalkFrame({ run, wide = false, children }: { run: string; host: HTMLElement; wide?: boolean; children: ReactNode }) {
  return (
    <div role="dialog" aria-label={`Bridge's window on ${run}`} className="armada-mock-walk-window__frame" data-panel={wide ? "" : undefined}>
      {children}
    </div>
  );
}

type EntryProps = { run: string; url: string; host: HTMLElement } & WalkWindowOptions & {
  /** The window's queue, where it holds more than one walk. */
  queue?: CaptureBarQueue;
  /** The Job whose view the panel draws. Absent: no panel. */
  panelFor?: string;
  /** Approve was taken: the walk is done and the window moves on. */
  onSettled?: () => void;
  /** Where the walks are named, and so how many rows the bar draws. */
  layout?: CaptureBarLayout;
  /** What the Job asked about this page: the bar says so, and one press draws the question over the page. */
  asking?: string;
  /** The question was answered, in these words. */
  onAsked?: (said: string) => void;
};

/** One walk on screen: its strip, its bar, its page and the panel beside it. A new walk is a new one of these, so nothing of the last is kept. */
function WalkEntry({ run, url: first, host, job, into = "Job", title, address, onOpenOwner, onNote, onApprove, variations, readiness: opened, queue: row, panelFor, onSettled, layout, asking, onAsked }: EntryProps) {
  const [readiness, setReadiness] = useState(opened);
  useEffect(() => {
    mark = setReadiness;
    return () => {
      mark = null;
    };
  }, []);
  const testing = (import.meta as { env?: { MODE?: string } }).env?.MODE === "test";
  const frame = useRef<HTMLIFrameElement>(null);
  const [armed, setArmed] = useState(false);
  const [picked, setPicked] = useState<Picked | null>(null);
  const [said, setSaid] = useState("");
  const [kept, setKept] = useState(0);
  // Approving one take approves it alone: another on screen is Approve again, and pressing it moves the approval.
  const [approval, setApproval] = useState<{ at: number; state: CaptureBarApproval } | undefined>(undefined);
  const [at, setAt] = useState(0);
  const [panelOpen, setPanelOpen] = useState(true);
  const [questioning, setQuestioning] = useState(false);
  const [reply, setReply] = useState("");
  const walking = useWalkOn(frame, testing);
  const url = variations?.[at]?.url ?? first;
  const taken: Taken | undefined = variations?.[at] === undefined ? undefined : { letter: variationLetter(at), name: variations[at].name };

  // **Bridge dims behind the window while it has focus**, as main dims its own
  // windows behind a focused walk window: a press here or in its page takes
  // focus, and a press anywhere on the app behind gives it back.
  useEffect(() => {
    const dim = (on: boolean) => document.documentElement.toggleAttribute("data-walking", on);
    const pressed = (event: PointerEvent) => dim(host.contains(event.target as Node));
    // The page inside the frame takes focus without a press reaching this document.
    const left = () => queueMicrotask(() => dim(document.activeElement === frame.current));
    dim(true);
    document.addEventListener("pointerdown", pressed, true);
    window.addEventListener("blur", left);
    return () => {
      dim(false);
      document.removeEventListener("pointerdown", pressed, true);
      window.removeEventListener("blur", left);
    };
  }, [host]);

  // The chord, from this page and from the page inside the frame — whose own
  // annotation layer it reaches first, being the frame's earliest listener
  // only after load, so it claims it there too.
  useEffect(() => {
    claimChord = () => setArmed((was) => !was);
    const toggle = (event: KeyboardEvent) => {
      if (!isChord(event)) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      setArmed((was) => !was);
    };
    const inner = frame.current;
    const watchInner = () => inner?.contentWindow?.addEventListener("keydown", toggle, true);
    watchInner();
    inner?.addEventListener("load", watchInner);
    return () => {
      claimChord = null;
      inner?.contentWindow?.removeEventListener("keydown", toggle, true);
      inner?.removeEventListener("load", watchInner);
    };
  }, []);

  // The queue's keys reach the bar wherever the cursor is in this page but a field, as main hands a window's bar the keys it
  // hears with nothing of its own focused. A field keeps ⌘↵ as its Send.
  useEffect(() => {
    if (row === undefined) return undefined;
    const keyed = (event: KeyboardEvent) => {
      const step = (event.key === "ArrowRight" || event.key === "ArrowLeft") && event.metaKey && event.altKey && !event.ctrlKey && !event.shiftKey;
      const approve = event.key === "Enter" && event.metaKey && !event.altKey && !event.ctrlKey && !event.shiftKey;
      const target = event.target;
      if (!event.isTrusted || !(step || approve) || !(target instanceof HTMLElement) || host.contains(target)) return;
      if (target.isContentEditable || target.closest("input, textarea, select") !== null) return;
      const bar = host.querySelector(".armada-capture-bar");
      if (bar === null) return;
      event.preventDefault();
      event.stopPropagation();
      const { key, metaKey, altKey, ctrlKey, shiftKey } = event;
      bar.dispatchEvent(new KeyboardEvent("keydown", { key, metaKey, altKey, ctrlKey, shiftKey, bubbles: true, cancelable: true }));
    };
    window.addEventListener("keydown", keyed, true);
    return () => window.removeEventListener("keydown", keyed, true);
  }, [row === undefined, host]);

  // Armed: the page is held still, the element under the pointer is ringed,
  // and a press picks it rather than acting on it.
  useEffect(() => {
    const page = frame.current?.contentDocument;
    if (!armed || page == null) return;
    let ringed: HTMLElement | null = null;
    const ring = (on: HTMLElement | null) => {
      if (ringed !== null) ringed.style.outline = "";
      ringed = on;
      // The framed page is this mock, so its own tokens draw the ring.
      if (on !== null) on.style.outline = "var(--focus-ring)";
    };
    const over = (event: MouseEvent) => ring(event.target as HTMLElement);
    const press = (event: MouseEvent) => {
      event.preventDefault();
      event.stopPropagation();
      const target = event.target as HTMLElement;
      setPicked({
        element: described(target),
        selector: selectorOf(target),
        location: page.location.pathname + page.location.search,
        ...(taken === undefined ? {} : { variation: taken }),
      });
      setArmed(false);
    };
    page.addEventListener("mouseover", over, true);
    page.addEventListener("click", press, true);
    return () => {
      ring(null);
      page.removeEventListener("mouseover", over, true);
      page.removeEventListener("click", press, true);
    };
  }, [armed, at]);

  function save(): void {
    if (picked === null || said.trim() === "") return;
    onNote(said.trim(), picked);
    setKept((was) => was + 1);
    setPicked(null);
    setSaid("");
  }

  return (
    <>
      <div className="armada-mock-walk-window__strip" onPointerDown={dragging(host)}>
        <span>
          {run} — drag to move, corner to resize
          {kept === 0 ? "" : ` · ${kept} ${kept === 1 ? "note" : "notes"} sent to the Job`}
        </span>
        <Button variant="secondary" size="sm" onClick={closeWalkWindow}>
          Close window
        </Button>
      </div>
      <CaptureBar
        run={title === null ? undefined : (title ?? run)}
        address={address ?? (title === null ? url : new URL(url).origin)}
        studio={null}
        {...(into === "Session" ? { session: job } : { job })}
        // Bridge comes forward, as main raises its window over this one.
        onOpenOwner={() => {
          document.documentElement.removeAttribute("data-walking");
          onOpenOwner?.();
        }}
        serving
        armed={armed}
        framesRefused={0}
        onArm={setArmed}
        onReload={() => {
          if (frame.current !== null && !testing) frame.current.src = url;
        }}
        onFollowRefused={() => {}}
        binding={["⌥", "⌘", "A"]}
        {...(onApprove === undefined
          ? {}
          : {
              onApprove: () => {
                const pressed = at;
                setApproval({ at: pressed, state: { state: "pending" } });
                void onApprove(url, taken).then((answer) => {
                  setApproval({ at: pressed, state: answer.ok ? { state: "approved" } : { state: "refused", said: answer.said } });
                  if (answer.ok) onSettled?.();
                });
              },
            })}
        {...(approval?.at !== at ? {} : { approval: approval.state })}
        {...(variations === undefined ? {} : { variations: { names: variations.map((one) => one.name), at, onPick: setAt } })}
        {...(readiness === undefined ? {} : { readiness })}
        {...(walking === null ? {} : { walk: walking })}
        {...(row === undefined ? {} : { queue: row })}
        {...(layout === undefined ? {} : { layout })}
        {...(asking === undefined ? {} : { question: asking, onShowQuestion: () => setQuestioning(true) })}
        {...(panelFor === undefined ? {} : { panel: { kind: "session" as const, open: panelOpen, onToggle: setPanelOpen } })}
      />
      {asking === undefined || !questioning ? null : (
        <WindowQuestion
          from={job}
          question={asking}
          options={[]}
          answer={reply}
          onAnswer={setReply}
          onChoose={() => {}}
          onSend={() => onAsked?.(reply.trim())}
          onFold={() => setQuestioning(false)}
        />
      )}
      {picked === null ? null : (
        <form
          className="armada-mock-walk-window__note"
          aria-label="Note on what you picked"
          onSubmit={(event) => {
            event.preventDefault();
            save();
          }}
        >
          <span>
            On {picked.variation === undefined ? "" : `${picked.variation.letter}, `}
            {picked.element}
          </span>
          <textarea
            aria-label="What is wrong here"
            value={said}
            autoFocus
            onChange={(event) => setSaid(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && event.metaKey) save();
              if (event.key === "Escape") setPicked(null);
            }}
          />
          <div>
            <Button variant="ghost" size="sm" type="button" onClick={() => setPicked(null)}>
              Cancel
            </Button>
            <Button size="sm" type="submit" disabled={said.trim() === ""}>
              Send to the {into}
            </Button>
          </div>
        </form>
      )}
      <div className="armada-mock-walk-window__body">
        <iframe ref={frame} title={`${run}, served`} src={testing ? "about:blank" : url} data-armed={armed || undefined} />
        {layout === "panel" && row !== undefined ? (
          // The walks are named at the head of the panel, which keeps them when the Job's view is folded away.
          <div className="armada-mock-walk-window__panel">
            <CaptureQueueList queue={row} />
            {panelFor === undefined || !panelOpen ? null : <div ref={publishPanelHost} className="armada-mock-walk-window__job" />}
          </div>
        ) : panelFor === undefined || !panelOpen ? null : (
          <div ref={publishPanelHost} className="armada-mock-walk-window__panel" />
        )}
      </div>
    </>
  );
}

/**
 * The walk in the framed page, as the bar draws it: **null where the page is not a walk**. A page
 * that loads again — a walk replaying for Back — is read again when it has loaded. The card is told to
 * leave the page to the bar, and to come back when the bar goes.
 *
 * **No page under test**, so a test gets a walk of its own over three steps: the bar's row and its keys
 * are what it holds.
 */
function useWalkOn(frame: RefObject<HTMLIFrameElement | null>, testing: boolean): CaptureBarWalk | null {
  const [handle, setHandle] = useState<WalkHandle | null>(null);
  const [state, setState] = useState<WalkState | null>(null);

  useEffect(() => {
    const inner = frame.current;
    if (inner === null) return;
    let poll = 0;
    let off = (): void => {};
    const connect = (): void => {
      off();
      off = () => {};
      window.clearInterval(poll);
      setHandle(null);
      setState(null);
      let tries = 0;
      const look = (): boolean => {
        // A page of another origin, or none yet, has no handle to read.
        let found: WalkHandle | undefined;
        try {
          found = testing ? testWalk : inner.contentWindow?.__armadaWalk;
        } catch {
          return false;
        }
        if (found === undefined) return false;
        found.hostCard(true);
        const hear = found.subscribe(() => setState(found.state()));
        off = () => {
          hear();
          try {
            found.hostCard(false);
          } catch {
            // The page is gone with its card.
          }
        };
        setHandle(found);
        setState(found.state());
        return true;
      };
      // The walk mounts a moment after the page loads.
      if (!look()) poll = window.setInterval(() => (look() || (tries += 1) > 50) && window.clearInterval(poll), 100);
    };
    inner.addEventListener("load", connect);
    connect();
    return () => {
      inner.removeEventListener("load", connect);
      window.clearInterval(poll);
      off();
    };
  }, [frame, testing]);

  if (handle === null || state === null) return null;
  return {
    name: state.name,
    at: state.at,
    of: state.of,
    said: state.said,
    autoplay: state.autoplay,
    ready: state.ready,
    done: state.done,
    onBack: handle.back,
    onNext: handle.next,
    onAutoplay: handle.autoplay,
    onReplay: handle.replay,
  };
}

/** A walk over three steps held in memory, for a window with no page in its frame. */
const testWalk: WalkHandle = (() => {
  const said = ["First step", "Second step", "Third step"];
  const hearing = new Set<() => void>();
  let at = 0;
  let autoplay = false;
  const move = (to: number) => {
    at = to;
    hearing.forEach((hear) => hear());
  };
  let snapshot: WalkState = { name: "a walk", at: 1, of: 3, said: said[0]!, autoplay, ready: true, done: false };
  const state = () => {
    if (snapshot.at !== Math.min(at + 1, 3) || snapshot.autoplay !== autoplay) {
      snapshot = { name: "a walk", at: Math.min(at + 1, 3), of: 3, said: said[Math.min(at, 2)]!, autoplay, ready: at < 3, done: at >= 3 };
    }
    return snapshot;
  };
  return {
    state,
    subscribe: (hear) => {
      hearing.add(hear);
      return () => void hearing.delete(hear);
    },
    next: () => at < 3 && move(at + 1),
    back: () => at > 0 && move(at - 1),
    autoplay: (on) => {
      autoplay = on;
      move(at);
    },
    replay: () => move(0),
    hostCard: () => {},
  };
})();

/** What was picked, the way a person names it: its role or tag and its words. */
function described(element: HTMLElement): string {
  const role = element.getAttribute("role") ?? element.tagName.toLowerCase();
  const words = (element.getAttribute("aria-label") ?? element.textContent ?? "").trim().replace(/\s+/g, " ");
  return words === "" ? role : `${role} “${words.length > 48 ? `${words.slice(0, 47)}…` : words}”`;
}

/** A path of tags and classes down to it, short enough to read. */
function selectorOf(element: HTMLElement): string {
  const parts: string[] = [];
  for (let at: HTMLElement | null = element; at !== null && parts.length < 4; at = at.parentElement) {
    const first = [...at.classList][0];
    parts.unshift(first === undefined ? at.tagName.toLowerCase() : `${at.tagName.toLowerCase()}.${first}`);
  }
  return parts.join(" > ");
}

/** Move the window by its strip. The frame is under the pointer too, so it is told to let go of it. */
function dragging(host: HTMLElement) {
  return (down: React.PointerEvent<HTMLDivElement>) => {
    if ((down.target as HTMLElement).closest("button") !== null) return;
    const frame = host.querySelector<HTMLElement>(".armada-mock-walk-window__frame");
    if (frame === null) return;
    const from = frame.getBoundingClientRect();
    const start = { x: down.clientX - from.left, y: down.clientY - from.top };
    frame.style.pointerEvents = "none";
    const move = (at: PointerEvent) => {
      frame.style.left = `${Math.max(0, at.clientX - start.x)}px`;
      frame.style.top = `${Math.max(0, at.clientY - start.y)}px`;
      frame.style.right = "auto";
    };
    const up = () => {
      frame.style.pointerEvents = "";
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };
}
