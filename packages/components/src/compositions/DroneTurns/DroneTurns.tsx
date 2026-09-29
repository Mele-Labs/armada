import { Fragment, useEffect, useId, useRef, useState, type ReactNode } from "react";
import { ChevronDown, ChevronRight, CircleDot } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { ACTOR_NAMED, type ActivityActor } from "../ActivityLog/ActivityLog";

/**
 * Drone turns — one Drone's transcript, read while it is still being written.
 *
 * **Read-only, and it must not read as Pilot.** Observing changes nothing about
 * the Job: no status moves, no transition is recorded, the Drone is never told.
 * So nothing in this component takes a control, and a row carries no act —
 * `docs/concepts/observe.md` is the table that separates the two. The one
 * control here reveals rows this pane already holds.
 *
 * **A call and its answer are one row.** They arrive as two events with the
 * tool running in the gap between them, and two rows would separate a command
 * from its output by everything that happened while it ran. The join is on the
 * call id and it happens here rather than in Fleet, where holding a call open
 * to wait for its result would be unbounded buffering in the loop that advances
 * the Job.
 *
 * **A card per speaker.** Consecutive turns from one speaker — Drone, Armada or
 * Fleet — are one card, headed with the speaker's name in the words
 * `ActivityLog` names them with, and a change of speaker starts the next card.
 * The rows inside carry only what happened: a call leads with its tool, prose
 * is plain, and a refusal says so in its body. The owner drew it on 29 Sep
 * 2026, replacing the column that named the speaker on every row: *"it just
 * repeats pretty much the same thing on every row."* A turn with no speaker is
 * a card of its own with no head, rather than one under a guessed name.
 *
 * **Tool calls read as a quiet block under the sentences.** Consecutive calls
 * gather into one block — smaller, muted, set in behind a rule — so reading
 * down a card gives what the Drone said, with its work tucked between. The
 * owner, 29 Sep 2026: *"the lines all just kind of blend together."*
 *
 * **Times are a card's, not a row's.** The head carries the card's first
 * instant; each row keeps its own as its title.
 *
 * **The step is a boundary, not a column.** One step's turns run to dozens, so
 * a name repeated down every row would be the same string forty times over
 * competing for the width the body absorbs. The question a reader asks is where
 * the step changed, and a line drawn there, between cards, answers it for every
 * row beneath. A step changing under one speaker splits that speaker's card.
 *
 * **It follows the tail, and stops the moment you scroll away from it.** A
 * pane that pulls you back to the bottom while you are reading is worse than
 * one that never moves. It resumes when you return to the bottom.
 */
export type DroneTurn = {
  /** Stable across re-renders. Rows arrive in order and nothing reorders them. */
  id: string;
  /** When Fleet's line loop saw it, not when it reached the disk. */
  at: string;
  /** The wire's kind: `called`, `refused`, `said`, and the rest. Not drawn; it gathers calls. */
  kind: string;
  /** Who wrote the row. Names its card; absent, the row is a headless card of its own. */
  who?: ActivityActor;
  /** The machine value the row is about — a tool, a session, an unread line. */
  subject?: string;
  /**
   * What the call did: the command run, the path read, the pattern searched.
   * Rendered where the wire carries it and **never derived from the tool
   * name** — a tool name alone is what made twenty-two rows read alike.
   */
  detail?: ReactNode;
  /** `detail` was cut short upstream. Rendered as cut, never as the whole value. */
  truncated?: boolean;
  /**
   * What came back, where it says more than that the call answered — a plain
   * successful answer carries none. Absent draws nothing.
   */
  answer?: ReactNode;
  /** Prose: the Drone's own text, or the harness's wording for a refusal. */
  said?: ReactNode;
  /**
   * The row is the Drone thinking rather than something it did. Consecutive
   * quiet rows collapse to one line, expandable, keeping the count.
   *
   * Measured on one real transcript: 106 of 149 rows. Naming them by the
   * decoder's failure to place them described the plumbing; from the reader's
   * side it is a model working, which is what the collapsed line says.
   */
  quiet?: boolean;
  /**
   * The workflow step the row ran under. Drawn as a boundary above the first
   * row of each run of it, never on the row itself.
   *
   * **Absent is not the first step.** It is a row written before Fleet recorded
   * which step was running, and nothing can recover which it was — so the
   * boundary above such rows says they carry none rather than naming one.
   */
  step?: TurnStep;
};

/** One step, named the way the rail names one: Fleet's label, or Fleet's id. */
export type TurnStep = {
  /** The `step_id`. What one run of a step is told from the next by. */
  id: string;
  /** The step's name, in sans. The `step_id` where no name is known. */
  label: string;
  /** Whether `label` is the `step_id` rather than a name, so it renders in mono. */
  labelIsAnIdentifier?: boolean;
};

export type DroneTurnsProps = {
  /** In the order Fleet sent them. History first, then live rows. */
  turns: DroneTurn[];
  /**
   * What the pane says with no rows at all. A Job that was never dispatched is
   * ordinary rather than an error, and the sentence has to say which.
   */
  emptyNote: string;
  /**
   * Whether a Drone is writing. Decides whether the trailing collapsed run
   * says so and its mark moves — a finished transcript showing a live mark on
   * a gap in its middle would claim work that stopped.
   */
  live?: boolean;
};

export function DroneTurns({ turns, emptyNote, live = false }: DroneTurnsProps) {
  const [open, setOpen] = useState<ReadonlySet<string>>(() => new Set());
  // State rather than a ref, because every effect below needs the element and
  // the first render has no rows: the socket says whether a Drone is writing
  // after the pane is already on screen.
  const [list, setList] = useState<HTMLOListElement | null>(null);
  const following = useRef(false);
  const seeded = useRef(false);
  const drawn = useRef(0);
  const scroller = useRef<Element | null>(null);

  // **`live` decides where the pane opens, once.** A live run opens at the tail
  // and a finished one at the top, where the brief is — landing at the bottom of
  // a whole history hides its beginning. A run that ends under a reader changes
  // nothing about where they are.
  useEffect(() => {
    if (list === null || seeded.current) return;
    seeded.current = true;
    following.current = live;
  }, [list, live]);

  // The pane owns no scroller: it is drawn inside one, so it reads the ancestor
  // that scrolls rather than declaring a height no drawing gives it.
  useEffect(() => {
    const found = scrollerFor(list);
    scroller.current = found;
    if (list === null || found === null) return undefined;
    // A viewport's scroll event is fired at the document and does not reach the
    // element that scrolled, so the listener goes on the window instead.
    const target = found === document.scrollingElement ? window : found;
    const read = (): void => {
      following.current = atBottom(found);
    };
    target.addEventListener("scroll", read, { passive: true });
    return () => target.removeEventListener("scroll", read);
  }, [list]);

  // **The scroller's own bottom, not the list's.** `scrollIntoView` would stop
  // at the list's last row, leaving the region's padding below it — which the
  // scroll this causes then reads as having been scrolled away from, so the
  // pane would follow exactly once. `scrollTop` clamps, so this lands where
  // `atBottom` measures. Nothing animates, so nothing is reduced.
  useEffect(() => {
    if (list === null || turns.length === drawn.current) return;
    drawn.current = turns.length;
    const pane = scroller.current;
    if (!following.current || pane === null) return;
    pane.scrollTop = pane.scrollHeight;
  }, [list, turns.length]);

  if (turns.length === 0) {
    return (
      <p className="armada-turns__empty" role="note">
        {emptyNote}
      </p>
    );
  }

  const entries = runs(turns);
  const tail = entries[entries.length - 1];
  return (
    <ol className="armada-turns" ref={setList}>
      {entries.map((entry) =>
        entry.of === "step" ? (
          <StepBoundary key={`step-${entry.above}`} step={entry.step} />
        ) : (
          <Card key={firstOf(entry).id} card={entry}>
            {entry.items.map((item, at) =>
              item.of === "turn" ? (
                <Row key={item.turn.id} turn={item.turn} />
              ) : item.of === "calls" ? (
                <Calls key={item.turns[0].id} turns={item.turns} />
              ) : (
                <QuietRun
                  key={item.turns[0].id}
                  turns={item.turns}
                  // Only the last run can still be happening: a run with rows
                  // after it already ended.
                  working={live && entry === tail && at === entry.items.length - 1}
                  open={open.has(item.turns[0].id)}
                  onToggle={() => setOpen(toggled(open, item.turns[0].id))}
                />
              ),
            )}
          </Card>
        ),
      )}
    </ol>
  );
}

/**
 * How near the bottom still counts as the bottom, in pixels.
 *
 * **Never an equality test.** `scrollTop` is fractional on a HiDPI display
 * while `scrollHeight` and `clientHeight` are rounded, so the three never
 * cancel exactly and a pane compared against zero stops following for good.
 * Four absorbs that rounding and is far under one row, so scrolling away by
 * even a line still reads as scrolling away.
 */
const BOTTOM_SLACK = 4;

function atBottom(scroller: Element): boolean {
  return scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight <= BOTTOM_SLACK;
}

/** The nearest ancestor that scrolls, or the document's own scroller. */
function scrollerFor(from: Element | null): Element | null {
  for (let at = from?.parentElement ?? null; at !== null; at = at.parentElement) {
    const overflow = getComputedStyle(at).overflowY;
    if (overflow === "auto" || overflow === "scroll") return at;
  }
  return document.scrollingElement;
}

type Item =
  | { of: "turn"; turn: DroneTurn }
  /**
   * At least one row, always: a run is opened with the turn that started it
   * and only ever grows. Written as a non-empty tuple so the head below is a
   * turn rather than a lookup every caller has to assert.
   */
  | { of: "quiet"; turns: [DroneTurn, ...DroneTurn[]] }
  /** Consecutive calls, refused ones included. Non-empty for the same reason. */
  | { of: "calls"; turns: [DroneTurn, ...DroneTurn[]] };

/** One speaker's consecutive turns. Never empty, for the same reason as a run. */
type CardEntry = { of: "card"; who?: ActivityActor; items: [Item, ...Item[]] };

type Entry =
  | CardEntry
  /** Keyed by the row it stands above, which is stable while rows only append. */
  | { of: "step"; step?: TurnStep; above: string };

/**
 * The rows, gathered into a card per run of one speaker, with consecutive quiet
 * rows gathered inside it and each change of step marked between cards.
 *
 * **A run of one is still a run.** Left alone it renders the decoder's own
 * words for a turn it could not place, which is the reading this collapse
 * exists to remove, and one line that reads like its neighbours beats one that
 * does not.
 *
 * **A boundary breaks a card and a run**, because either spanning two steps
 * would attribute the whole of it to whichever the reader guessed.
 *
 * **A turn with no speaker never shares a card**, not even with another
 * unnamed one: two rows under one head says one speaker wrote both.
 *
 * **Nothing is marked where no row anywhere carries a step.** Every row of such
 * a transcript predates the field, so "not recorded" would be the only line on
 * screen and would contrast with nothing. A transcript that gains a step
 * part-way through says so at the point it does.
 */
function runs(turns: DroneTurn[]): Entry[] {
  const attributed = turns.some((turn) => turn.step !== undefined);
  const entries: Entry[] = [];
  let under: string | undefined;
  let opened = false;
  let card: CardEntry | undefined;
  for (const turn of turns) {
    if (attributed && (!opened || turn.step?.id !== under)) {
      entries.push({ of: "step", step: turn.step, above: turn.id });
      under = turn.step?.id;
      opened = true;
      card = undefined;
    }
    const item: Item =
      turn.quiet === true
        ? { of: "quiet", turns: [turn] }
        : isCall(turn)
          ? { of: "calls", turns: [turn] }
          : { of: "turn", turn };
    if (card === undefined || card.who === undefined || card.who !== turn.who) {
      card = { of: "card", ...(turn.who === undefined ? {} : { who: turn.who }), items: [item] };
      entries.push(card);
      continue;
    }
    const last = card.items[card.items.length - 1];
    if ((item.of === "quiet" || item.of === "calls") && last?.of === item.of) {
      last.turns.push(turn);
      continue;
    }
    card.items.push(item);
  }
  return entries;
}

/** By kind, never by shape: a session line carries a subject alone, as a bare call does. */
function isCall(turn: DroneTurn): boolean {
  return turn.kind === "called" || turn.kind === "refused";
}

function toggled(open: ReadonlySet<string>, id: string): ReadonlySet<string> {
  const next = new Set(open);
  if (!next.delete(id)) next.add(id);
  return next;
}

/** The mark is 12px at strokeWidth 2, the step mark's geometry one level down. */
const MARK = 12;
const MARK_STROKE = 2;
const CARET = 16;

type QuietRunProps = {
  /** Non-empty, for `Entry`'s reason: the head is what the folded line reads. */
  turns: [DroneTurn, ...DroneTurn[]];
  working: boolean;
  open: boolean;
  onToggle: () => void;
};

/**
 * A run of quiet rows as one line, and the rows themselves when it is opened.
 *
 * **The rows are never dropped, only folded.** This pane already tells a viewer
 * when the backfill skipped rows and when a slow viewer lost some; hiding rows
 * with no trace would contradict both, and a pane that cannot be trusted to
 * hold everything cannot answer what the Drone did.
 */
function QuietRun({ turns, working, open, onToggle }: QuietRunProps) {
  const head = turns[0];
  // Only while they exist. `aria-controls` naming ids that are not in the
  // document is an invalid value, not an empty one.
  const held = open ? turns.map((turn) => rowId(turn)).join(" ") : undefined;
  return (
    <Fragment>
      <li
        className="armada-turns__turn"
        data-quiet
        data-open={open || undefined}
        title={head.at}
      >
        <span className="armada-turns__mark" data-working={working || undefined}>
          <CircleDot size={MARK} strokeWidth={MARK_STROKE} aria-hidden />
        </span>
        <span className="armada-turns__quiet-body">
          {/* Live only. A finished transcript is a record, and a record does
              not narrate: once nothing is happening the count is the whole
              fact and the still mark already says the run ended. */}
          {working ? <span className="armada-turns__working">{"Working"}</span> : null}
          <span className="armada-turns__count">{counted(turns.length)}</span>
          <Button
            variant="ghost"
            size="sm"
            aria-expanded={open}
            aria-controls={held}
            onClick={onToggle}
          >
            {open ? (
              <ChevronDown size={CARET} strokeWidth={MARK_STROKE} aria-hidden />
            ) : (
              <ChevronRight size={CARET} strokeWidth={MARK_STROKE} aria-hidden />
            )}
            {open ? "Hide details" : "Show details"}
          </Button>
        </span>
      </li>
      {open ? turns.map((turn) => <Row key={turn.id} turn={turn} nested />) : null}
    </Fragment>
  );
}

/** The turn a card opens with, whose instant its head carries. */
function firstOf(card: CardEntry): DroneTurn {
  const head = card.items[0];
  return head.of === "turn" ? head.turn : head.turns[0];
}

/** Consecutive calls as one quiet block, a list named for what it holds. */
function Calls({ turns }: { turns: [DroneTurn, ...DroneTurn[]] }) {
  return (
    <li className="armada-turns__calls">
      <ol className="armada-turns__call-rows" aria-label="Tool calls">
        {turns.map((turn) => (
          <Row key={turn.id} turn={turn} />
        ))}
      </ol>
    </li>
  );
}

/**
 * One speaker's run of turns: the name over a rule, then the rows.
 *
 * **A group named by its speaker**, so a card is found by who wrote it rather
 * than by its head's styling. A card with no speaker has no head and no name —
 * nothing is guessed for it.
 */
function Card({ card, children }: { card: CardEntry; children: ReactNode }) {
  const named = useId();
  return (
    <li className="armada-turns__card">
      <div role="group" {...(card.who === undefined ? {} : { "aria-labelledby": named })}>
        {card.who === undefined ? null : (
          <div className="armada-turns__card-head">
            <span className="armada-turns__who" id={named}>
              {ACTOR_NAMED[card.who]}
            </span>
            <span className="armada-turns__at">{firstOf(card).at}</span>
          </div>
        )}
        <ol className="armada-turns__rows">{children}</ol>
      </div>
    </li>
  );
}

/**
 * What rows written before Fleet recorded a step say for themselves.
 *
 * **Never the first step.** That is the falsehood a transcript told before the
 * wire carried this at all: a four-step Job claiming the whole of it happened
 * under step one. The true step is unrecoverable, so this says so.
 */
const UNRECORDED = "Fleet recorded no step for the turns below";

/**
 * Where the step changed, as a line across the transcript.
 *
 * **No glyph.** `circle-*` belongs to Judge verdicts, `shield-*` to Checks and
 * `file*` to evidence, and a boundary is none of those — the rule and the name
 * carry it, and a mark spent here would be a mark meaning something else.
 *
 * The word `step` leads it in mono, which gives it an accessible name that says
 * what it is without hidden text.
 */
function StepBoundary({ step }: { step?: TurnStep }) {
  return (
    <li
      className="armada-turns__step"
      data-unrecorded={step === undefined || undefined}
      data-identifier={step?.labelIsAnIdentifier || undefined}
    >
      <span className="armada-turns__kind armada-turns__step-tag">{"step"}</span>
      <span className="armada-turns__step-name">{step?.label ?? UNRECORDED}</span>
    </li>
  );
}

function Row({ turn, nested = false }: { turn: DroneTurn; nested?: boolean }) {
  return (
    <li
      className="armada-turns__turn"
      id={rowId(turn)}
      data-nested={nested || undefined}
      title={turn.at}
    >
      <span className="armada-turns__body">
        {turn.subject === undefined && turn.detail === undefined ? null : (
          <span className="armada-turns__head">
            {turn.subject === undefined ? null : (
              <span className="armada-turns__subject">{turn.subject}</span>
            )}
            {turn.detail === undefined ? null : (
              <span className="armada-turns__detail">
                {turn.detail}
                {turn.truncated === true ? <Cut /> : null}
              </span>
            )}
          </span>
        )}
        {turn.answer === undefined ? null : (
          <span className="armada-turns__answer">{turn.answer}</span>
        )}
        {turn.said === undefined ? null : <span className="armada-turns__said">{turn.said}</span>}
      </span>
    </li>
  );
}

/**
 * A value the wire cut short. **Never a tooltip**, which the contract gives
 * anything truncated in a row: there is no fuller value anywhere to put in one,
 * so the ellipsis is the whole fact and the note says it in words.
 */
function Cut() {
  return (
    <span className="armada-turns__cut">
      {"…"}
      <span className="armada-turns__cut-note">{" cut short"}</span>
    </span>
  );
}

function rowId(turn: DroneTurn): string {
  return `armada-turn-${turn.id}`;
}

/** `70 turns`, and `1 turn`. */
function counted(rows: number): string {
  return rows === 1 ? "1 turn" : `${rows} turns`;
}
