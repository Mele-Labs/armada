// Helm's conversation zone, under the dock's questions — #944. `App.tsx` only
// wires this in; which repository Helm answers for is main's own decision,
// made in `main/helm.ts`, and this draws whatever it publishes.
//
// It carries Helm's own permission asks too, since #1519: a Drone's question
// and a Judge's stay in the block above, and an ask that stopped this
// conversation is drawn in it.

import { useRef, useState, type KeyboardEvent } from "react";
import { copyHelmRecord, DockQuestions, HelmComposer, HelmRecord, HelmThread, ProseRefs, ThoughtCard } from "@armada/components";
import { useRecentRepositories } from "@armada/shell";
import type {
  DockQuestion,
  HelmApprovalCard,
  HelmApprovalCardState,
  HelmBox,
  HelmComposerChip,
  HelmThreadRow,
  ProseReferences,
  SentFromComposer,
} from "@armada/components";
import type {
  HelmContext,
  HelmDebugInfo,
  HelmDebugRead,
  JobSummary,
  RepositorySummary,
  WorkflowSummary,
} from "@armada/protocol";
import { helmRowsOf, type HelmApprovalAsk, type HelmFoldedRow } from "./helm-thread";
import type { HelmState } from "./api";
import { locationOf, type StudioNamed } from "./helm-context";
import { recentManifestIds } from "./recent-manifests";
import { routeTargetOf, type RouteTarget, type ThoughtCarry, type ThoughtReading, type ThoughtRouting } from "./thought-routing";

export type HelmDockProps = {
  helm: HelmState["helm"];
  repositories: readonly RepositorySummary[];
  /** Resolved into a card's Job facts — never anything the model itself said. #1041. */
  jobs: readonly JobSummary[];
  workflows: readonly WorkflowSummary[];
  live: boolean;
  /** The open Job's chip, while it stands — `App.tsx`'s own state. #1075. */
  chip?: HelmComposerChip;
  onRemoveChip?: () => void;
  onAsk: (text: string, context: HelmContext) => void;
  /** Where the person is, assembled by `App.tsx`. Sent with every ask, unchanged here. #1075. */
  context: HelmContext;
  /** The open Studio and its selected node, by name, for the footer. #1287. */
  studio?: StudioNamed;
  onSwitch: (manifestId: string) => void;
  /** The session as one record, read once when a person opens it — #1367. */
  onReadRecord: () => Promise<HelmDebugRead>;
  /** Told after the record's clipboard write, either way. */
  onCopied: (what: string) => void;
  /** Told why the record could not be read, where nothing could be copied. */
  onSaid: (sentence: string) => void;
  /** Approve, answered: the card waits on this, and a refusal puts it back to ready. #1117. */
  onApprove: (jobId: string) => Promise<{ ok: boolean }>;
  /**
   * Every permission ask this session is held inside, already worded by
   * `dockQuestionsOf` — #1519. Drawn under the thread and over the message
   * box, which is where the reply it stopped is and where the person is
   * looking. Oldest first, and empty draws nothing.
   */
  asks?: readonly DockQuestion[];
  /**
   * Where a thought may go. **Present, Helm reads every message first**: a question is asked as
   * ever, and a thought for other work gets a card over the message box. Absent, every message is
   * an ask. Words sent on to other work come back in `helm` as a row of their own (`thought-routing.ts`).
   */
  routing?: ThoughtRouting;
  /**
   * The Sessions box's parts: what `/`, `@` and `#` offer, the model and effort, and the sketches
   * waiting. Built by the window from the Sessions it holds. Absent, the box offers only what is typed.
   */
  box?: Partial<HelmBox>;
  /**
   * What the thread's words name that Bridge can open: pull requests, issues, Jobs and Sessions, in
   * Helm's replies and in the person's own rows. Absent, the thread is plain text.
   */
  refs?: ProseReferences;
};

/**
 * A thought the person sent: being read, matched to running work, or offered a new home. **They
 * queue in the order typed**, and the first one is the card in front.
 */
type Thought = { id: number; text: string; carry: ThoughtCarry } & (
  | { stage: "routing" }
  | { stage: "match"; match: RouteTarget }
  | { stage: "new" }
);

/**
 * A card's own press, held for this dock's lifetime alone — a reload starts
 * over. #1041. `"pending"` is Approve pressed and Fleet not yet answered. #1117.
 */
type Pressed = "pending" | "approved" | "dismissed";

export function HelmDock({
  helm,
  repositories,
  jobs,
  workflows,
  live,
  chip,
  onRemoveChip,
  onAsk,
  context,
  studio,
  onSwitch,
  onReadRecord,
  onCopied,
  onSaid,
  onApprove,
  asks = [],
  routing,
  box,
  refs,
}: HelmDockProps) {
  // What each message went with, for the thread's row to draw: Fleet's row for an ask carries words alone.
  const [carried, setCarried] = useState<{ text: string; carry: ThoughtCarry }[]>([]);
  const [thoughts, setThoughts] = useState<Thought[]>([]);
  // The thought in front is the oldest one still held; the rest wait their turn behind it.
  const thought = thoughts[0];
  // Held while words are being delivered, so a second press of ↵ cannot send them twice.
  const delivering = useRef(false);
  // A thought's number is its place in the queue; one answered or dismissed leaves `held`, so its reading is let go.
  const counted = useRef(0);
  const held = useRef(new Set<number>());
  const [record, setRecord] = useState<Read>({ open: false });
  const [pressed, setPressed] = useState<Record<string, Pressed>>({});
  const current = helm.state === "none" ? undefined : helm.manifestId;
  const [recentRoots, remember] = useRecentRepositories();
  const options = repositories
    .filter((one): one is RepositorySummary & { manifest: NonNullable<RepositorySummary["manifest"]> } =>
      one.manifest !== undefined,
    )
    .map((one) => ({ id: one.manifest.id, label: one.manifest.repository }));
  const recent = recentManifestIds(repositories, recentRoots);
  function switchTo(manifestId: string): void {
    const picked = repositories.find((one) => one.manifest?.id === manifestId);
    if (picked !== undefined) remember(picked.root);
    onSwitch(manifestId);
  }
  const folded = helm.state === "open" || helm.state === "failed" ? helmRowsOf(helm.items) : [];
  const rows: HelmThreadRow[] = withCarry(
    folded.map((row) => withCards(row, jobs, workflows, pressed, onApprove, setPressed, repositories, routing)),
    carried,
  );
  const replying = helmReplying(helm);

  const notice = !live
    ? "Fleet is not connected. What Helm already said is still here."
    : helm.state === "failed"
      ? helm.detail
      : undefined;

  /**
   * Read the record, then draw it. **Asked each time**, never held: it is one
   * artifact taken at a moment, and a stale one quoted into an issue is worse
   * than none.
   */
  function openRecord(): void {
    setRecord({ open: true });
    void onReadRecord().then((read) =>
      setRecord(read.ok ? { open: true, record: read.record } : { open: true, failed: whyRecord(read) }),
    );
  }

  /**
   * One press, and the record is on the clipboard — the banner form: this is
   * reached when Helm has just answered badly and somebody wants to carry it
   * now, and the reading is what they want second.
   *
   * **The read is between the press and the write**, so a Fleet that would not
   * answer must say so rather than leave a silent control: a failed clipboard
   * write and a dead control look the same, and so does a copy that never had
   * anything to copy.
   */
  function copyRecord(): void {
    void onReadRecord().then((read) =>
      read.ok ? copyHelmRecord(read.record, onCopied) : onSaid(whyRecord(read)),
    );
  }

  function send(sent: SentFromComposer): void {
    const text = sent.text.trim();
    const carry: ThoughtCarry = { files: sent.files, tags: sent.tags, sketches: box?.drawn ?? [] };
    const goesWith = carry.files.length + carry.tags.length + carry.sketches.length > 0;
    if (text === "" && !goesWith) return;
    // Sketches went with this message and do not wait for the next.
    carry.sketches.forEach((one) => box?.onRemoveDrawn?.(one.id));
    if (goesWith) setCarried((was) => [...was, { text, carry }]);
    if (routing === undefined) return onAsk(text, context);
    const id = (counted.current += 1);
    held.current.add(id);
    setThoughts((was) => [...was, { id, text, carry, stage: "routing" }]);
    // A read that cannot be had is a question: the words go to Helm and are never lost.
    const asQuestion = (): ThoughtReading => ({ kind: "question" });
    void Promise.resolve()
      .then(() => routing.read(text))
      .catch(asQuestion)
      .then((reading) => {
        if (!held.current.has(id)) return;
        // **A question never waits behind a card.** It goes to Helm as soon as it is read.
        if (reading.kind === "question") {
          leave(id);
          onAsk(text, context);
        } else {
          change(id, reading.match === undefined ? { id, text, carry, stage: "new" } : { id, text, carry, stage: "match", match: reading.match });
        }
      });
  }

  function change(id: number, next: Thought): void {
    setThoughts((was) => was.map((one) => (one.id === id ? next : one)));
  }

  /** A thought is answered or let go: the next one, if any, is the card in front. */
  function leave(id: number): void {
    held.current.delete(id);
    setThoughts((was) => was.filter((one) => one.id !== id));
  }

  /** The words go to Helm as an ordinary message, since the guess at where they belong can be wrong. */
  function askHelm(one: Thought): void {
    leave(one.id);
    onAsk(one.text, context);
  }

  /**
   * Deliver the words. **The card stays until they have arrived**, and its row is the thread's: it
   * is drawn when Fleet's record of the delivery reaches `helm`, so a refusal leaves the card for
   * another try, for N or for H, and nothing is shown as sent that was not.
   */
  function place(one: Thought & { stage: "match" }): void {
    if (routing === undefined || delivering.current) return;
    delivering.current = true;
    void routing
      .send(one.match, one.text, one.carry)
      .catch(() => false)
      .then((delivered) => {
        delivering.current = false;
        if (delivered && held.current.has(one.id)) leave(one.id);
      });
  }

  function startNew(kind: "session" | "job", one: Thought): void {
    leave(one.id);
    routing?.startNew(kind, one.text);
  }

  /**
   * The card's keys, while the message box is empty: from then on a letter is an answer and not a
   * word. A button that has focus keeps ↵ for itself, so pressing it never answers twice.
   */
  function keyed(event: KeyboardEvent<HTMLDivElement>): void {
    if (thought === undefined || thought.stage === "routing" || filled(event.currentTarget)) return;
    if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) return;
    const key = event.key.toLowerCase();
    const onButton = event.target instanceof HTMLElement && event.target.closest("button") !== null;
    const act =
      key === "escape"
        ? () => leave(thought.id)
        : key === "h"
          ? () => askHelm(thought)
          : thought.stage === "match"
            ? key === "enter" && !onButton
              ? () => place(thought)
              : key === "n"
                ? () => change(thought.id, { id: thought.id, text: thought.text, carry: thought.carry, stage: "new" })
                : undefined
            : key === "s"
              ? () => startNew("session", thought)
              : key === "j"
                ? () => startNew("job", thought)
                : undefined;
    if (act === undefined) return;
    event.preventDefault();
    event.stopPropagation();
    act();
  }

  return (
    <div className="armada-helm-dock" onKeyDownCapture={keyed}>
      <ProseRefs.Provider value={refs ?? null}>
        <HelmThread rows={rows} replying={replying} notice={notice} />
      </ProseRefs.Provider>
      {/* Between the thread and the message box: the reply is held open above
          it and the person is typing below it, so the ask is in the one place
          neither of them has to be left to find it — #1519. The same
          `DockQuestions` the dock draws, so there is one card and not two. */}
      <DockQuestions questions={asks} />
      {thought === undefined ? null : thought.stage === "routing" ? (
        <ThoughtCard key={thought.id} stage="routing" words={thought.text} />
      ) : thought.stage === "match" ? (
        <ThoughtCard
          key={thought.id}
          stage="match"
          words={thought.text}
          target={thought.match}
          onSend={() => place(thought)}
          onNo={() => change(thought.id, { id: thought.id, text: thought.text, carry: thought.carry, stage: "new" })}
          onAskHelm={() => askHelm(thought)}
        />
      ) : (
        <ThoughtCard
          key={thought.id}
          stage="new"
          words={thought.text}
          onSession={() => startNew("session", thought)}
          onJob={() => startNew("job", thought)}
          onAskHelm={() => askHelm(thought)}
          onDismiss={() => leave(thought.id)}
        />
      )}
      <HelmComposer
        current={current}
        repositories={options}
        recent={recent}
        chip={chip}
        onRemoveChip={onRemoveChip}
        location={locationOf(context, jobs, studio)}
        // Always passed: **the composer alone decides when a switch is worth
        // drawing**, off `current` and this list. Counting the repositories
        // here as well is how one set up and Helm pointed at nothing kept
        // drawing no control under a sentence naming one. A specific pick
        // does not hide it either — Discuss, or the switch itself, points
        // Helm away from the picked repository without moving the rail.
        onSwitch={switchTo}
        onCopyRecord={current === undefined ? undefined : copyRecord}
        onOpenRecord={current === undefined ? undefined : openRecord}
        {...(box === undefined ? {} : { box })}
        onSend={send}
        disabled={!live || current === undefined}
      />
      <HelmRecord
        open={record.open}
        record={record.record}
        failed={record.failed}
        onCopied={onCopied}
        onClose={() => setRecord({ open: false })}
      />
    </div>
  );
}

/** Whether the message box holds anything: from then on a letter is a word and not an answer to the card. */
function filled(dock: HTMLElement): boolean {
  return (dock.querySelector('[role="textbox"]')?.textContent ?? "") !== "";
}

/**
 * The rows of the messages that went with something, drawn with it. **Matched on the words, newest
 * first**: each entry belongs to the latest row of yours that says them and has none yet.
 */
function withCarry(rows: HelmThreadRow[], carried: readonly { text: string; carry: ThoughtCarry }[]): HelmThreadRow[] {
  if (carried.length === 0) return rows;
  const given = new Map<string, ThoughtCarry>();
  for (let at = carried.length - 1; at >= 0; at -= 1) {
    const one = carried[at]!;
    const row = [...rows].reverse().find((it) => it.actor === "you" && it.message === one.text && !given.has(it.id));
    if (row !== undefined) given.set(row.id, one.carry);
  }
  return rows.map((row) => {
    const carry = given.get(row.id);
    return carry === undefined ? row : { ...row, files: carry.files, sketches: carry.sketches, tags: carry.tags };
  });
}

/**
 * A reply is being written, or a message is waiting for one — the one rule
 * *Start fresh* is refused by, and Fleet's rather than a guess drawn here.
 * **Exported because the act left this component**: the head of the dock now
 * carries it (`App.tsx`), and the thread below still reads the same fact, so
 * the two must not be able to disagree about it.
 */
export function helmReplying(helm: HelmState["helm"]): boolean {
  return helm.state === "open" && helm.replying;
}

/** The record sheet's own state, held for as long as it is open and no longer. */
type Read = { open: boolean; record?: HelmDebugInfo; failed?: string };

/**
 * Why the record could not be read, in one sentence. **Fleet's own refusal
 * where it gave one**, and the connection's where the request never left.
 */
function whyRecord(read: Extract<HelmDebugRead, { ok: false }>): string {
  const outcome = read.outcome;
  if (outcome.ok) return "The session could not be read.";
  if (outcome.why === "not_connected") return "Fleet is not connected, so the session cannot be read.";
  if (outcome.why === "refused") return outcome.error.message;
  return "The session could not be read.";
}

/**
 * One folded row's bare `asks` resolved into drawable cards, off `jobs` and
 * `workflows` — the app's own published state, never anything Helm's tool
 * call said. `#1041`.
 */
function withCards(
  row: HelmFoldedRow,
  jobs: readonly JobSummary[],
  workflows: readonly WorkflowSummary[],
  pressed: Record<string, Pressed>,
  onApprove: (jobId: string) => Promise<{ ok: boolean }>,
  setPressed: (update: (was: Record<string, Pressed>) => Record<string, Pressed>) => void,
  repositories: readonly RepositorySummary[],
  routing: ThoughtRouting | undefined,
): HelmThreadRow {
  const { asks, sent, ...rest } = row;
  // Words that went to other work: the repository is named as the picker names it, and the chip opens the Session or Job.
  const target = sent === undefined ? undefined : routeTargetOf(sent, repositories);
  const placed = target === undefined ? {} : { sent: { target, onOpen: () => routing?.open(target) } };
  if (asks === undefined || asks.length === 0) return { ...rest, ...placed };
  return {
    ...rest,
    ...placed,
    cards: asks.map((ask) => cardOf(ask, jobs, workflows, pressed, onApprove, setPressed)),
  };
}

function cardOf(
  ask: HelmApprovalAsk,
  jobs: readonly JobSummary[],
  workflows: readonly WorkflowSummary[],
  pressed: Record<string, Pressed>,
  onApprove: (jobId: string) => Promise<{ ok: boolean }>,
  setPressed: (update: (was: Record<string, Pressed>) => Record<string, Pressed>) => void,
): HelmApprovalCard {
  const job = jobs.find((one) => one.id === ask.jobId);
  const workflow = job === undefined ? undefined : workflows.find(matching(job));
  const state = stateOf(ask.id, job, pressed);
  return {
    id: ask.id,
    jobHandle: job?.handle ?? ask.jobId,
    workflow: workflow?.name,
    stepCount: workflow?.steps.length,
    state,
    ...(state === "ready" || state === "pending"
      ? {
          onApprove: () => {
            setPressed((was) => ({ ...was, [ask.id]: "pending" }));
            // Refused: drop the press, so the card reads the Board again and can be pressed again.
            void onApprove(ask.jobId).then((answer) =>
              setPressed(({ [ask.id]: _pending, ...rest }) =>
                answer.ok ? { ...rest, [ask.id]: "approved" } : rest,
              ),
            );
          },
          onDismiss: () => setPressed((was) => ({ ...was, [ask.id]: "dismissed" })),
        }
      : {}),
  };
}

function matching(job: JobSummary): (workflow: WorkflowSummary) => boolean {
  return (workflow) => workflow.id === job.workflow_id && workflow.manifest_id === job.owner_manifest_id;
}

/**
 * **A local press wins over the Board once it has been made** — so a card
 * this dock approved reads "Approved." rather than snapping straight to
 * `elsewhere` the instant the Job leaves `awaiting_approval` because the
 * press worked, and one still waiting on Fleet reads `pending`.
 *
 * Absent any press, the Board's own status decides: a Job no longer there, or
 * one this card never moved, reads `elsewhere`; a Job this dock has never
 * heard of reads `unknown`, in words rather than a broken card.
 */
function stateOf(id: string, job: JobSummary | undefined, pressed: Record<string, Pressed>): HelmApprovalCardState {
  const local = pressed[id];
  if (local !== undefined) return local;
  if (job === undefined) return "unknown";
  return job.status === "awaiting_approval" ? "ready" : "elsewhere";
}
