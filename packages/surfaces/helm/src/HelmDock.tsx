// Helm's conversation zone, under the dock's questions — #944. `App.tsx` only
// wires this in; which repository Helm answers for is main's own decision,
// made in `main/helm.ts`, and this draws whatever it publishes.
//
// It carries Helm's own permission asks too, since #1519: a Drone's question
// and a Judge's stay in the block above, and an ask that stopped this
// conversation is drawn in it.

import { useRef, useState, type KeyboardEvent } from "react";
import { copyHelmRecord, DockQuestions, HelmComposer, HelmRecord, HelmThread, ThoughtCard } from "@armada/components";
import { clock } from "@armada/screens/src/duration";
import { useRecentRepositories } from "@armada/shell";
import type {
  DockQuestion,
  HelmApprovalCard,
  HelmApprovalCardState,
  HelmComposerChip,
  HelmThreadRow,
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
import type { RouteTarget, ThoughtRouting } from "./thought-routing";

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
   * an ask. Mock-only for now (`thought-routing.ts`).
   */
  routing?: ThoughtRouting;
};

/** The thought in front of the person: being read, matched to running work, or offered a new home. */
type Thought = { id: number; text: string } & (
  | { stage: "routing" }
  | { stage: "match"; match: RouteTarget }
  | { stage: "new" }
);

/** Words sent to running work instead of Helm, kept in the thread at the place they were sent. */
type Placed = { id: number; after: number; at: string; text: string; target: RouteTarget };

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
}: HelmDockProps) {
  const [draft, setDraft] = useState("");
  const [thought, setThought] = useState<Thought>();
  const [placed, setPlaced] = useState<Placed[]>([]);
  // Bumped by every message and every dismissal, so a reading for words no longer in front is let go.
  const latest = useRef(0);
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
  const asked = folded.map((row) => withCards(row, jobs, workflows, pressed, onApprove, setPressed));
  const rows: HelmThreadRow[] = [];
  const inPlace = (at: (after: number) => boolean) =>
    placed
      .filter((one) => at(one.after))
      .forEach((one) =>
        rows.push({
          id: `thought-${one.id}`,
          at: one.at,
          actor: "you",
          message: one.text,
          sent: { target: one.target, onOpen: () => routing?.open(one.target) },
        }),
      );
  asked.forEach((row, index) => {
    inPlace((after) => after === index);
    rows.push(row);
  });
  inPlace((after) => after >= asked.length);
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

  function send(): void {
    const text = draft.trim();
    if (text === "") return;
    setDraft("");
    if (routing === undefined) return onAsk(text, context);
    const id = (latest.current += 1);
    setThought({ id, text, stage: "routing" });
    void routing.read(text).then((reading) => {
      if (latest.current !== id) return;
      if (reading.kind === "question") {
        setThought(undefined);
        onAsk(text, context);
      } else {
        setThought(reading.match === undefined ? { id, text, stage: "new" } : { id, text, stage: "match", match: reading.match });
      }
    });
  }

  function dismiss(): void {
    latest.current += 1;
    setThought(undefined);
  }

  /** The words go to Helm as an ordinary message, since the guess at where they belong can be wrong. */
  function askHelm(text: string): void {
    dismiss();
    onAsk(text, context);
  }

  function place(one: Thought & { stage: "match" }): void {
    routing?.send(one.match, one.text);
    setPlaced((was) => [
      ...was,
      { id: one.id, after: folded.length, at: clock(new Date().toISOString()), text: one.text, target: one.match },
    ]);
    dismiss();
  }

  function startNew(kind: "session" | "job", text: string): void {
    dismiss();
    routing?.startNew(kind, text);
  }

  /**
   * The card's keys, while the message box is empty: from then on a letter is an answer and not a
   * word. A button that has focus keeps ↵ for itself, so pressing it never answers twice.
   */
  function keyed(event: KeyboardEvent<HTMLDivElement>): void {
    if (thought === undefined || thought.stage === "routing" || draft !== "") return;
    if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) return;
    const key = event.key.toLowerCase();
    const onButton = event.target instanceof HTMLElement && event.target.closest("button") !== null;
    const act =
      key === "escape"
        ? dismiss
        : key === "h"
          ? () => askHelm(thought.text)
          : thought.stage === "match"
            ? key === "y" || (key === "enter" && !onButton)
              ? () => place(thought)
              : key === "n"
                ? () => setThought({ id: thought.id, text: thought.text, stage: "new" })
                : undefined
            : key === "s"
              ? () => startNew("session", thought.text)
              : key === "j"
                ? () => startNew("job", thought.text)
                : undefined;
    if (act === undefined) return;
    event.preventDefault();
    event.stopPropagation();
    act();
  }

  return (
    <div className="armada-helm-dock" onKeyDownCapture={keyed}>
      <HelmThread rows={rows} replying={replying} notice={notice} />
      {/* Between the thread and the message box: the reply is held open above
          it and the person is typing below it, so the ask is in the one place
          neither of them has to be left to find it — #1519. The same
          `DockQuestions` the dock draws, so there is one card and not two. */}
      <DockQuestions questions={asks} />
      {thought === undefined ? null : thought.stage === "routing" ? (
        <ThoughtCard stage="routing" words={thought.text} />
      ) : thought.stage === "match" ? (
        <ThoughtCard
          stage="match"
          words={thought.text}
          target={thought.match}
          onSend={() => place(thought)}
          onNo={() => setThought({ id: thought.id, text: thought.text, stage: "new" })}
          onAskHelm={() => askHelm(thought.text)}
        />
      ) : (
        <ThoughtCard
          stage="new"
          words={thought.text}
          onSession={() => startNew("session", thought.text)}
          onJob={() => startNew("job", thought.text)}
          onAskHelm={() => askHelm(thought.text)}
          onDismiss={dismiss}
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
        value={draft}
        onChange={setDraft}
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
): HelmThreadRow {
  const { asks, ...rest } = row;
  if (asks === undefined || asks.length === 0) return rest;
  return {
    ...rest,
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
