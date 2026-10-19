import { Check, ChevronLeft, ChevronRight, MessageCircleQuestion, RotateCcw, Search, UserCheck } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent, ReactNode } from "react";

import { Button } from "../../primitives/Button/Button";
import { DropdownMenu, type DropdownMenuEntry } from "../../primitives/DropdownMenu/DropdownMenu";
import { Kbd, KbdChord } from "../../primitives/Kbd/Kbd";
import { Switch } from "../../primitives/Switch/Switch";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { SessionMark } from "../SessionFrame/SessionFrame";

/**
 * The bar Bridge owns above another repository's running web app —
 * `docs/practices/capture-window.md`, *What a person sees*; `#1294`.
 *
 * **A person has to tell at a glance that what is below this is not Armada.**
 * So it names the Run, the address in full and the Studio a Note will land on,
 * and it is drawn outside the page rather than in it — a bar composited above
 * is a bar the page cannot draw over.
 *
 * **There is no address field, no Back and no Forward.** This is not a browser:
 * the window is pinned to one origin and Reload reloads that one.
 */

/**
 * Where an Approve press stands. `refused` carries Fleet's words, drawn as the
 * button's tooltip so the bar keeps its one row.
 */
export type CaptureBarApproval = { state: "pending" } | { state: "approved" } | { state: "refused"; said: string };

/** An address this window refused, as the bar says it. */
export type CaptureBarRefusal = {
  address: string;
  /** What moved: a navigation, a redirect, a popup or a download. */
  said: string;
  /** Whether the system browser would take it. */
  offerable: boolean;
};

export type CaptureBarProps = {
  /**
   * What the Manifest calls the server this window is on. **Absent where the window has no name of
   * its own** and was titled by its address: the address says it, and the bar does not say it twice.
   */
  run?: string | undefined;
  /** Scheme, host and port, drawn in full and never abbreviated. The whole address where there is no `run`. */
  address: string;
  /**
   * The Studio a Note lands on, by name. **`null`: a server opened to be
   * walked** — the bar says so, and offers no Capture, since a Note would have
   * nowhere to land.
   */
  studio: string | null;
  /** The Session a note goes to, where a Session showed this page. */
  session?: string;
  /**
   * Open the Session named in "Notes go to", in Bridge's main window. **Absent: the name
   * is plain text.**
   */
  onOpenOwner?: () => void;
  /** Whether the Run is still serving. Capture ends with it. */
  serving: boolean;
  /** Whether capture is armed. */
  armed: boolean;
  /** How many subframe navigations were refused. Counted, never named. */
  framesRefused: number;
  refused?: CaptureBarRefusal;
  onArm: (on: boolean) => void;
  onReload: () => void;
  /** Hand the refused address to the browser this person already uses. */
  onFollowRefused: () => void;
  /** The binding that arms capture, from the action registry. */
  binding: readonly string[];
  /**
   * Tell the Session that opened this window that the person walked
   * it and it is right. **Absent: no Approve**, as on a Studio's window, which
   * has no owner to tell.
   */
  onApprove?: () => void;
  approval?: CaptureBarApproval;
  /**
   * A question the Session asked about this page, folded out of the way so the page can be used. The
   * second row says it is waiting and `onShowQuestion` draws it over the page again.
   */
  question?: string;
  onShowQuestion?: () => void;
  /**
   * The panel beside the page, holding the Session's thread. **Absent:
   * a Studio's window, which has none.** One press draws it or folds it away.
   */
  panel?: CaptureBarPanel;
  /**
   * The takes a Session showed of one change, each a page of its own. One is drawn at a time; a
   * press swaps the page, and Approve and every note name the one on screen. **Absent: one page.**
   */
  variations?: CaptureBarVariations;
  /**
   * Whether the page is ready to walk, as the Session that opened it says: `ready` it is
   * done and waiting on the person, `working` it is still changing it. Drawn as the Session mark at
   * the bar's start and as the bar's own hue. **Absent: nobody has said.**
   */
  readiness?: CaptureBarReadiness;
  /**
   * The walk the page is playing, its controls drawn in a row of their own so the page keeps its
   * room. **Absent: the page is not a walk.** Keys pressed on the bar act on the walk as they do in
   * the page.
   */
  walk?: CaptureBarWalk;
  /**
   * The walks this window holds, oldest first, the one on screen among them. ⌥⌘→, ⌥⌘← and ⌘↵ act on
   * it, and `layout` says where the walks are named. **Absent: the window holds the one walk.**
   */
  queue?: CaptureBarQueue;
  /**
   * **At most two rows, in every state.** The first holds the bar and the takes, the second the walk's steps
   * and the one status that wants saying; a walk with neither draws no second row. Where the walks are named
   * is the take: `tabs` names them in the first row, `switcher` as a menu on the Job's name, and `panel` leaves
   * them to `CaptureQueueList` in the panel beside the page. **Absent: today's rows.**
   */
  layout?: CaptureBarLayout;
};

export type CaptureBarLayout = "tabs" | "switcher" | "panel";

/** One walk in the window's queue. */
export type CaptureBarQueued = {
  id: string;
  /** The Job's own name, or a pull request's title after its number. */
  name: string;
  /** A pull request's walk: the number, drawn before `name` as `#<n>`. */
  pull?: number;
  /** The window's title, the first line of the entry's tooltip. */
  title: string;
  /** The page's address, in full, and who its notes go to: the rest of the tooltip. */
  address: string;
  aim?: string;
  /** Ready to walk or still changing, as the entry's mark. */
  readiness?: CaptureBarReadiness;
  /** What the Job asked of the person about this page, where it did. A walk that is not on screen carries a mark and the page stays. */
  asking?: string;
};

export type CaptureBarQueue = {
  entries: readonly CaptureBarQueued[];
  /** The entry on screen, by position. */
  at: number;
  onPick: (at: number) => void;
  /** ⌥⌘→ is `1` and ⌥⌘← is `-1`. */
  onStep: (by: 1 | -1) => void;
};

/** The queue's keys, drawn beside what they act on. ⌘↵ is Approve's only while no field holds the cursor, where it sends. */
export const QUEUE_KEYS = { next: ["⌥", "⌘", "→"], previous: ["⌥", "⌘", "←"], approve: ["⌘", "↵"] } as const;

/** Whether a key lands in a field, where ⌘↵ is the field's own Send. */
const inAField = (target: EventTarget | null): boolean =>
  target instanceof HTMLElement && (target.isContentEditable || target.closest("input, textarea, select") !== null);

/** The queue's keys as the bar takes them. True when the key was one of them. */
function queueKey(event: KeyboardEvent<HTMLElement>, queue: CaptureBarQueue, approve: (() => void) | undefined): boolean {
  const act =
    event.key === "ArrowRight" && event.metaKey && event.altKey && !event.ctrlKey && !event.shiftKey
      ? () => queue.onStep(1)
      : event.key === "ArrowLeft" && event.metaKey && event.altKey && !event.ctrlKey && !event.shiftKey
        ? () => queue.onStep(-1)
        : event.key === "Enter" && event.metaKey && !event.altKey && !event.ctrlKey && !event.shiftKey && !inAField(event.target)
          ? approve
          : undefined;
  if (act === undefined) return false;
  event.preventDefault();
  event.stopPropagation();
  act();
  return true;
}

/** Where a walk on the page stands, and the acts on it. `at` counts from 1. */
export type CaptureBarWalk = {
  name: string;
  at: number;
  of: number;
  /** The step's caption. */
  said: string;
  autoplay: boolean;
  /** The step's target is ringed, so Next has something to act on. */
  ready: boolean;
  /** Past the last step. */
  done: boolean;
  onBack: () => void;
  onNext: () => void;
  onAutoplay: (on: boolean) => void;
  onReplay: () => void;
  /**
   * Present where Bridge plays the steps itself and draws the ring over the page. `shown` is whether
   * the ring is up, `onShow` puts it away or draws it again so the page can be used, `stopped` names
   * the target the step cannot find, and `signingIn` is the page being on a provider's, which the walk
   * waits out. `waiting` is Autoplay stopped before a step that acts on a page off this machine, so
   * Next is what the walk waits on; signing in wins over it. **Absent: the page plays its own walk** and
   * rings its own target.
   */
  guide?: { shown: boolean; onShow: (shown: boolean) => void; stopped?: string; signingIn?: boolean; waiting?: boolean };
};

/**
 * The walk's keys, as the page takes them: a key that is not one of them is left alone. Enter
 * is left to a button of the bar's own row above, which it presses.
 */
function walkKey(event: KeyboardEvent<HTMLElement>, walk: CaptureBarWalk): void {
  if (event.metaKey || event.ctrlKey || event.altKey) return;
  const key = event.key.toLowerCase();
  const onAButton = event.target instanceof HTMLElement && event.target.closest("button") !== null;
  const inWalkRow = event.target instanceof HTMLElement && event.target.closest(".armada-capture-bar__walk") !== null;
  const act =
    key === "arrowright" || (key === "enter" && (!onAButton || inWalkRow))
      ? walk.onNext
      : key === "arrowleft"
        ? walk.onBack
        : key === "a"
          ? () => walk.onAutoplay(!walk.autoplay)
          : key === "r"
            ? walk.onReplay
            : null;
  if (act === null) return;
  // Not the button's own press as well, and not the page behind that hears the same key.
  event.preventDefault();
  event.stopPropagation();
  act();
}

export type CaptureBarReadiness = "ready" | "working";

/** Each readiness as the Session mark draws it, and what its tooltip says. */
const READINESS: Record<CaptureBarReadiness, { mark: "waiting" | "working"; said: string }> = {
  ready: { mark: "waiting", said: "Ready to walk" },
  working: { mark: "working", said: "Still changing" },
};

/** Each take's name in the order shown, which is on screen, and the press that swaps it. */
export type CaptureBarVariations = { names: readonly string[]; at: number; onPick: (at: number) => void };

/** A take's letter: A for the first. */
export const variationLetter = (at: number): string => String.fromCharCode(65 + at);

/** Which panel the window has, whether it is drawn, and the press that changes that. */
export type CaptureBarPanel = { kind: "session"; open: boolean; onToggle: (open: boolean) => void };

/** Whether the bar has a second row to draw — a refusal, the run having ended, or a folded question. */
export function hasASecondRow(
  props: Pick<CaptureBarProps, "serving" | "refused" | "framesRefused" | "question">,
): boolean {
  return !props.serving || props.refused !== undefined || props.framesRefused > 0 || props.question !== undefined;
}

/** The walk's controls, the same wherever the bar draws them. */
function WalkControls({ walk }: { walk: CaptureBarWalk }) {
  return (
    <>
    <Tooltip label="Replay to the step before">
      <Button variant="ghost" size="sm" disabled={walk.at <= 1} onClick={walk.onBack}>
        <ChevronLeft size={12} strokeWidth={2} aria-hidden />
        Back
      </Button>
    </Tooltip>
    <Kbd>←</Kbd>
    <span className="armada-capture-bar__walk-where">
      Step {walk.at} of {walk.of}
    </span>
    {walk.guide?.signingIn !== true ? null : (
      <Tooltip label="Signing in">
        <span className="armada-capture-bar__walk-signing" role="img" aria-label="Signing in">
          <UserCheck size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
    )}
    {walk.guide?.stopped === undefined ? null : (
      <Tooltip label={`Not found: ${walk.guide.stopped}`}>
        <span className="armada-capture-bar__walk-stopped" role="img" aria-label="Not found">
          <Search size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
    )}
    <span className="armada-capture-bar__walk-said" title={walk.said}>
      {walk.said}
    </span>
    {walk.guide === undefined ? null : (
      <Tooltip label="Ring the target over the page">
        <Switch checked={walk.guide.shown} onChange={(event) => walk.guide?.onShow(event.target.checked)}>
          Ring
        </Switch>
      </Tooltip>
    )}
    <Tooltip label="Replay from step 1">
      <Button variant="ghost" size="sm" iconOnly aria-label="Replay" onClick={walk.onReplay}>
        <RotateCcw size={12} strokeWidth={2} aria-hidden />
      </Button>
    </Tooltip>
    <Kbd>R</Kbd>
    <Tooltip label="Go on by itself">
      <Switch checked={walk.autoplay} onChange={(event) => walk.onAutoplay(event.target.checked)}>
        Autoplay
      </Switch>
    </Tooltip>
    <Kbd>A</Kbd>
    <Tooltip label={walk.guide?.waiting === true && walk.guide.signingIn !== true ? "Autoplay stops before a step that acts on this page" : "Act on the ringed target and go on"}>
      <Button
        variant="primary"
        size="sm"
        disabled={!walk.ready || walk.done}
        onClick={walk.onNext}
        {...(walk.guide?.waiting === true && walk.guide.signingIn !== true ? { "data-waiting": "" } : {})}
      >
        Next
        <ChevronRight size={12} strokeWidth={2} aria-hidden />
      </Button>
    </Tooltip>
    <Kbd>→</Kbd>
    </>
  );
}

/** What an entry reads as: a Job by its name, a pull request by its number and title. */
const queuedName = (one: CaptureBarQueued): string => (one.pull === undefined ? one.name : `#${one.pull} ${one.name}`);

/** An entry's tooltip: the window's title, the address in full, who its notes go to, whether it is ready, and what it asked. */
function Tip({ one }: { one: CaptureBarQueued }) {
  const lines = [one.title, one.address, one.aim, one.readiness === undefined ? undefined : READINESS[one.readiness].said, one.asking === undefined ? undefined : `Asking: ${one.asking}`];
  return (
    <span className="armada-capture-bar__tip">
      {lines.flatMap((line, at) => (line === undefined ? [] : [<span key={at}>{line}</span>]))}
    </span>
  );
}

/** The ask mark: a registry glyph, and the question as its tooltip. */
function AskMark({ asking }: { asking: string }) {
  return (
    <Tooltip label={asking}>
      <span className="armada-capture-bar__queued-ask" role="img" aria-label="Asking">
        <MessageCircleQuestion size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/**
 * The walks waiting, one entry each, oldest first: a Job with two walks is named twice. **One stop for Tab, and the
 * arrows move along the entries** without opening anything; Enter opens the one under the cursor. The arrow keys with
 * ⌥⌘ held belong to the bar, which steps the page. `tabs` draws them as tabs along the bar, `list` as a short list.
 */
function QueueEntries({ queue, form }: { queue: CaptureBarQueue; form: "tabs" | "list" }) {
  const [roving, setRoving] = useState(queue.at);
  const presses = useRef<(HTMLButtonElement | null)[]>([]);
  // The stop follows the page, so Tab lands on the walk on screen.
  useEffect(() => setRoving(queue.at), [queue.at]);
  const last = queue.entries.length - 1;
  const tabs = form === "tabs";
  function move(event: KeyboardEvent<HTMLElement>): void {
    if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) return;
    const to =
      event.key === "ArrowRight" || event.key === "ArrowDown"
        ? Math.min(roving + 1, last)
        : event.key === "ArrowLeft" || event.key === "ArrowUp"
          ? Math.max(roving - 1, 0)
          : event.key === "Home"
            ? 0
            : event.key === "End"
              ? last
              : undefined;
    if (to === undefined) return;
    // Not the walk's own Next and Back, which hear the same arrows from the bar.
    event.preventDefault();
    event.stopPropagation();
    setRoving(to);
    presses.current[to]?.focus();
  }
  return (
    <div role={tabs ? "tablist" : "toolbar"} aria-label="Walks waiting" className="armada-capture-bar__entries" data-form={form} onKeyDown={move}>
      {queue.entries.map((one, at) => {
        const on = at === queue.at;
        return (
          <span key={one.id} role="presentation" className="armada-capture-bar__queued" data-on={on ? "" : undefined}>
            {one.readiness === undefined ? null : <SessionMark state={READINESS[one.readiness].mark} said={READINESS[one.readiness].said} />}
            <Tooltip label={<Tip one={one} />}>
              <button
                type="button"
                ref={(element) => {
                  presses.current[at] = element;
                }}
                {...(tabs ? { role: "tab", "aria-selected": on } : { "aria-current": on ? ("true" as const) : undefined })}
                className="armada-capture-bar__queued-press"
                tabIndex={at === roving ? 0 : -1}
                onFocus={() => setRoving(at)}
                onClick={() => queue.onPick(at)}
              >
                {queuedName(one)}
              </button>
            </Tooltip>
            {one.asking === undefined || on ? null : <AskMark asking={one.asking} />}
          </span>
        );
      })}
    </div>
  );
}

/** The two keys that step through the walks, drawn where the walks are named. */
function StepKeys() {
  return (
    <span className="armada-capture-bar__queue-keys">
      <KbdChord keys={[...QUEUE_KEYS.previous]} aria-label="⌥⌘← goes to the walk before" />
      <KbdChord keys={[...QUEUE_KEYS.next]} aria-label="⌥⌘→ goes to the next walk" />
    </span>
  );
}

/**
 * The queue as a short list, for the panel beside the page: the Job's name, its marks, the walk on screen selected.
 * Held in the panel so the bar keeps two rows; the keys step through it wherever the cursor is.
 */
export function CaptureQueueList({ queue }: { queue: CaptureBarQueue }) {
  return (
    <div className="armada-capture-queue">
      <QueueEntries queue={queue} form="list" />
      <StepKeys />
    </div>
  );
}

/** The takes of one change as a segmented control, for the bar's own row. */
function Takes({ variations }: { variations: CaptureBarVariations }) {
  return (
    <span role="radiogroup" aria-label="Variations" className="armada-capture-bar__segments">
      {variations.names.map((name, at) => (
        <button
          key={at}
          type="button"
          role="radio"
          aria-checked={at === variations.at}
          className="armada-capture-bar__take"
          onClick={() => variations.onPick(at)}
        >
          <span className="armada-capture-bar__take-letter">{variationLetter(at)}</span>
          {name}
        </button>
      ))}
    </span>
  );
}

/** The walks as a menu on the Job's name, the walk on screen chosen. A walk that asked carries the ask mark on its item and on the button. */
function QueueMenu({ queue }: { queue: CaptureBarQueue }) {
  const here = queue.entries[queue.at];
  const asker = queue.entries.find((one, at) => at !== queue.at && one.asking !== undefined);
  const entries: DropdownMenuEntry[] = [
    { kind: "label", id: "keys", label: "⌥⌘← ⌥⌘→ step through the walks" },
    ...queue.entries.map(
      (one, at): DropdownMenuEntry => ({
        kind: "item",
        id: one.id,
        label: queuedName(one),
        selected: at === queue.at,
        ...(one.asking !== undefined && at !== queue.at ? { icon: MessageCircleQuestion } : {}),
        hint: one.title,
      }),
    ),
  ];
  return (
    <span className="armada-capture-bar__switcher">
      <DropdownMenu
        plain
        align="start"
        triggerLabel={here === undefined ? "Walks" : queuedName(here)}
        triggerAriaLabel={`Walks, ${here === undefined ? "" : queuedName(here)}`}
        entries={entries}
        onSelect={(id) => queue.onPick(Math.max(0, queue.entries.findIndex((one) => one.id === id)))}
      />
      {asker?.asking === undefined ? null : <AskMark asking={asker.asking} />}
    </span>
  );
}

/**
 * The one status that wants saying, as a compact item with its action: the run ended, an address refused, a question waiting, or
 * frames refused, in that order. **Never a row of its own**: it sits at the end of the walk's row, or alone where there is no walk.
 */
function StatusItem(props: CaptureBarProps & { landsOn: string | null }) {
  const { serving, refused, question, framesRefused, landsOn } = props;
  if (!serving) {
    return (
      <StatusLine
        text="Run ended"
        tip={`This run is no longer serving. What is on screen stays and nothing further loads${landsOn === null ? "." : ", and capture is closed."}`}
        tone="ended"
      />
    );
  }
  if (refused !== undefined) {
    return (
      <StatusLine text={`${refused.said} refused`} tip={refused.address} tone="refused">
        {refused.offerable ? (
          <Button variant="ghost" size="sm" onClick={props.onFollowRefused}>
            Open in browser
          </Button>
        ) : null}
      </StatusLine>
    );
  }
  if (question !== undefined) {
    return (
      <StatusLine text={question} tip={question} tone="question" glyph={<MessageCircleQuestion size={12} strokeWidth={2} aria-hidden />}>
        <Button variant="secondary" ground="sunken" size="sm" onClick={props.onShowQuestion}>
          Show question
        </Button>
      </StatusLine>
    );
  }
  if (framesRefused > 0) return <StatusLine text="Frames refused" tip="Frames off this origin were refused" tone="frames" />;
  return null;
}

function StatusLine({ text, tip, tone, glyph, children }: { text: string; tip: string; tone: string; glyph?: ReactNode; children?: ReactNode }) {
  return (
    <span className="armada-capture-bar__status" role="status" data-tone={tone}>
      {glyph}
      <Tooltip label={tip}>
        <span className="armada-capture-bar__status-text">{text}</span>
      </Tooltip>
      {children}
    </span>
  );
}

export function CaptureBar(props: CaptureBarProps) {
  const { run, address, studio, session, serving, armed, framesRefused, refused, binding, approval, question, panel, variations, readiness, walk, queue, layout } = props;
  const taking = variations === undefined ? "" : ` ${variationLetter(variations.at)}`;
  const approved = approval?.state === "approved";
  const landsOn = studio !== null ? `Notes land on ${studio}` : session !== undefined ? `Notes go to ${session}` : null;
  const aim =
    studio === null && session !== undefined && props.onOpenOwner !== undefined ? (
      <>
        Notes go to{" "}
        <button
          type="button"
          className="armada-capture-bar__owner"
          title="Open this Job in Bridge"
          onClick={props.onOpenOwner}
        >
          {session}
        </button>
      </>
    ) : (
      landsOn
    );
  const reload = (
    <Button variant="ghost" size="sm" disabled={!serving} onClick={props.onReload}>
      Reload
    </Button>
  );
  const capture =
    landsOn === null ? null : (
      <>
        <Button variant={armed ? "primary" : "ghost"} size="sm" disabled={!serving} onClick={() => props.onArm(!armed)}>
          {armed ? "Capturing" : "Capture"}
        </Button>
        <KbdChord keys={[...binding]} aria-label={`${binding.join(" ")} turns capturing on`} />
      </>
    );
  const approve =
    props.onApprove === undefined ? null : (
      <>
        <Button
          variant="secondary"
          ground="sunken"
          size="sm"
          // While the opener is still changing the page there is nothing settled to approve.
          disabled={approval?.state === "approved" || readiness === "working"}
          pending={approval?.state === "pending"}
          {...(approval?.state === "refused" ? { answer: "refused" as const, title: approval.said } : {})}
          onClick={props.onApprove}
        >
          {readiness === "working" ? null : <Check size={12} strokeWidth={2} aria-hidden />}
          {readiness === "working" ? "Not ready" : approval?.state === "approved" ? `Approved${taking}` : `Approve${taking}`}
        </Button>
        {queue === undefined ? null : <KbdChord keys={[...QUEUE_KEYS.approve]} aria-label="⌘↵ approves" />}
      </>
    );
  const toggle =
    panel === undefined ? null : (
      <Button variant="ghost" size="sm" aria-expanded={panel.open} onClick={() => panel.onToggle(!panel.open)}>
        {`${panel.open ? "Hide" : "Show"} Job`}
      </Button>
    );
  const mark = approved ? (
    <span className="armada-capture-bar__approved">
      <Check size={14} strokeWidth={2.5} aria-hidden />
      {`Approved${taking}`}
    </span>
  ) : readiness === undefined ? null : (
    <SessionMark state={READINESS[readiness].mark} said={READINESS[readiness].said} />
  );
  const runChip =
    run === undefined ? null : (
      <span className="armada-capture-bar__run" title="This window shows a server, not Armada">
        {run}
      </span>
    );
  const root = {
    className: "armada-capture-bar",
    "data-armed": armed ? "" : undefined,
    // Approved outranks ready: the walk is settled, and the bar says so in its own hue.
    "data-readiness": approved ? undefined : readiness,
    "data-approved": approved ? "" : undefined,
    "data-layout": layout,
    ...(walk === undefined && queue === undefined
      ? {}
      : {
          onKeyDown: (event: KeyboardEvent<HTMLElement>) => {
            // Approve is not the key while the bar says there is nothing settled to approve.
            const act = props.onApprove === undefined || approved || readiness === "working" ? undefined : props.onApprove;
            if (queue !== undefined && queueKey(event, queue, act)) return;
            if (walk !== undefined) walkKey(event, walk);
          },
        }),
  };

  if (layout !== undefined) {
    const status = <StatusItem {...props} landsOn={landsOn} />;
    const hasStatus = !serving || refused !== undefined || question !== undefined || framesRefused > 0;
    const takes = variations === undefined ? null : <Takes variations={variations} />;
    return (
      <div {...root}>
        <div className="armada-capture-bar__row" data-layout={layout}>
          {layout === "tabs" && queue !== undefined ? (
            <>
              <QueueEntries queue={queue} form="tabs" />
              <StepKeys />
            </>
          ) : (
            <>
              {mark}
              {runChip}
              <span className="armada-capture-bar__address">{address}</span>
              <span className="armada-capture-bar__aim" {...(layout === "switcher" && queue !== undefined && landsOn !== null ? { "data-switcher": "" } : {})}>
                {!serving ? "The run ended" : layout === "switcher" && queue !== undefined && landsOn !== null ? (
                  <>
                    <span className="armada-capture-bar__aim-text">Notes go to</span>
                    <QueueMenu queue={queue} />
                  </>
                ) : (
                  (aim ?? "Walking a server")
                )}
              </span>
            </>
          )}
          <span className="armada-capture-bar__acts">
            {takes}
            {reload}
            {capture}
            {approve}
            {toggle}
          </span>
        </div>
        {walk === undefined && !hasStatus ? null : (
          <div className="armada-capture-bar__row armada-capture-bar__walk" {...(walk === undefined ? {} : { role: "group", "aria-label": `Walk ${walk.name}` })}>
            {walk === undefined ? null : <WalkControls walk={walk} />}
            {status}
          </div>
        )}
      </div>
    );
  }

  return (
    <div {...root}>
      <div className="armada-capture-bar__row">
        {mark}
        {runChip}
        <span className="armada-capture-bar__address">{address}</span>
        <span className="armada-capture-bar__aim">{!serving ? "The run ended" : (aim ?? "Walking a server")}</span>
        {reload}
        {capture}
        {approve}
        {toggle}
      </div>
      {/* A row of their own, so the takes never push the acts off a narrow window's bar. */}
      {variations === undefined ? null : (
        <div role="radiogroup" aria-label="Variations" className="armada-capture-bar__row armada-capture-bar__takes">
          {variations.names.map((name, at) => (
            <button
              key={at}
              type="button"
              role="radio"
              aria-checked={at === variations.at}
              className="armada-capture-bar__take"
              onClick={() => variations.onPick(at)}
            >
              <span className="armada-capture-bar__take-letter">{variationLetter(at)}</span>
              {name}
            </button>
          ))}
        </div>
      )}
      {walk === undefined ? null : (
        <div className="armada-capture-bar__row armada-capture-bar__walk" role="group" aria-label={`Walk ${walk.name}`}>
          <WalkControls walk={walk} />
        </div>
      )}
      {hasASecondRow(props) ? (
        <div className="armada-capture-bar__said" role="status">
          {serving ? null : (
            <span className="armada-capture-bar__ended">
              This run is no longer serving. What is on screen stays and nothing further loads
              {landsOn === null ? "." : ", and capture is closed."}
            </span>
          )}
          {refused === undefined ? null : (
            <>
              <span className="armada-capture-bar__refused">
                {refused.said} refused: {refused.address}
              </span>
              {refused.offerable ? (
                <Button variant="ghost" size="sm" onClick={props.onFollowRefused}>
                  Open in browser
                </Button>
              ) : null}
            </>
          )}
          {question === undefined ? null : (
            <>
              <span className="armada-capture-bar__question">Question waiting: {question}</span>
              <Button variant="secondary" ground="sunken" size="sm" onClick={props.onShowQuestion}>
                Show question
              </Button>
            </>
          )}
          {framesRefused > 0 ? (
            <span className="armada-capture-bar__frames">
              {framesRefused} {framesRefused === 1 ? "frame" : "frames"} off this origin refused
            </span>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
