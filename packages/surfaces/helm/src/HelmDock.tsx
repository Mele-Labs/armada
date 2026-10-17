// Helm's conversation zone, under the dock's questions — #944. `App.tsx` only
// wires this in; which repository Helm answers for is main's own decision,
// made in `main/helm.ts`. **Helm is a hosted Session**, so its thread is drawn as a Session's is: the
// window builds that thread and the chips for what Helm holds (`hosted`), and the dock lays them out
// and keeps what is Helm's own — the thought cards, the repository switch and the record. Before a
// repository's first message there is no Session and the thread is empty.
//
// It carries held asks too, since #1519: a Drone's question and a Judge's stay in the block above, and
// an ask that stopped a conversation is drawn under the thread.

import { useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { copyHelmRecord, DockQuestions, HelmComposer, HelmRecord, HelmThread, ProseRefs, ThoughtCard } from "@armada/components";
import { useRecentRepositories } from "@armada/shell";
import type {
  DockQuestion,
  HelmBox,
  ProseReferences,
  SentFromComposer,
} from "@armada/components";
import type {
  HelmContext,
  HelmDebugInfo,
  HelmDebugRead,
  RepositorySummary,
} from "@armada/protocol";
import type { HelmState } from "./api";
import { locationOf, type StudioNamed } from "./helm-context";
import { recentManifestIds } from "./recent-manifests";
import type { HeldWords, HelmOutgoing, RouteTarget, ThoughtCarry, ThoughtReading, ThoughtRouting } from "./thought-routing";

export type HelmDockProps = {
  helm: HelmState["helm"];
  repositories: readonly RepositorySummary[];
  live: boolean;
  /**
   * Words for Helm. `carry` is what went with them and is left out when nothing did; **the words may be
   * blank when it is given**. The window turns it into what the wire takes. Resolves to whether Fleet
   * took the words: a refusal is the thread's to show, on the message.
   */
  onAsk: (text: string, context: HelmContext, carry?: ThoughtCarry) => void | Promise<boolean>;
  /**
   * Where the words go in Helm's thread the moment they are sent, to be shown going. Absent, they are
   * not drawn until Fleet's own row arrives.
   */
  outgoing?: HelmOutgoing;
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
  /**
   * Every ask a conversation is held inside, already worded by
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
  /**
   * **The Helm Session's thread.** The window builds it as it builds a Session's, with its tool groups,
   * subagents and questions, and the chips for what Helm holds. Absent before the repository's first
   * message, and the thread is then empty.
   */
  hosted?: {
    thread: ReactNode;
    /** The slot, branch, pull requests and subagents Helm holds, under the dock's head. Absent draws none. */
    holds?: ReactNode;
    /** A turn is running. */
    working: boolean;
  };
};

/**
 * A thought the person sent: being read, matched to running work, or offered a new home. **They
 * queue in the order typed**, and the first one is the card in front.
 */
type Thought = { id: number; text: string; carry: ThoughtCarry; held?: HeldWords } & (
  | { stage: "routing" }
  | { stage: "match"; match: RouteTarget }
  | { stage: "new" }
);

export function HelmDock({
  helm,
  repositories,
  live,
  onAsk,
  outgoing,
  context,
  studio,
  onSwitch,
  onReadRecord,
  onCopied,
  onSaid,
  asks = [],
  routing,
  box,
  refs,
  hosted,
}: HelmDockProps) {
  const [thoughts, setThoughts] = useState<Thought[]>([]);
  // The thought in front is the oldest one still held; the rest wait their turn behind it.
  const thought = thoughts[0];
  // Held while words are being delivered, so a second press of ↵ cannot send them twice.
  const delivering = useRef(false);
  // A thought's number is its place in the queue; one answered or dismissed leaves `held`, so its reading is let go.
  const counted = useRef(0);
  const held = useRef(new Set<number>());
  const [record, setRecord] = useState<Read>({ open: false });
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
  const notice = live ? undefined : "Fleet is not connected. What Helm already said is still here.";

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

  /** Words for Helm, with what went with them when something did. The words in the thread are settled by the answer. */
  function ask(text: string, carry: ThoughtCarry, held?: HeldWords): void {
    const goes = onlyIfCarried(carry);
    const asked = () => (goes === undefined ? onAsk(text, context) : onAsk(text, context, goes));
    if (held === undefined) void asked();
    else void held.deliver(asked);
  }

  function send(sent: SentFromComposer): void {
    const text = sent.text.trim();
    const carry: ThoughtCarry = { files: sent.files, tags: sent.tags, sketches: box?.drawn ?? [] };
    const goesWith = carry.files.length + carry.tags.length + carry.sketches.length > 0;
    if (text === "" && !goesWith) return;
    // Sketches went with this message and do not wait for the next.
    carry.sketches.forEach((one) => box?.onRemoveDrawn?.(one.id));
    // In the thread from the press: while Helm reads where the words belong they are shown going, and no card stands in for them.
    const words = outgoing?.hold(text, carry);
    // Blank words are not read for where they belong: what they carry goes to Helm.
    if (routing === undefined || text === "") return ask(text, carry, words);
    const id = (counted.current += 1);
    held.current.add(id);
    setThoughts((was) => [...was, { id, text, carry, ...(words === undefined ? {} : { held: words }), stage: "routing" }]);
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
          ask(text, carry, words);
        } else {
          change(id, { id, text, carry, ...(words === undefined ? {} : { held: words }), ...(reading.match === undefined ? { stage: "new" as const } : { stage: "match" as const, match: reading.match }) });
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
    ask(one.text, one.carry, one.held);
  }

  /** The words were not sent anywhere: out of the thread, and the card goes with them. */
  function discard(one: Thought): void {
    one.held?.drop();
    leave(one.id);
  }

  /**
   * Deliver the words. **The card stays until they have arrived.** The words are in the thread meanwhile,
   * pulsing with the Sending mark, which is what keeps the promise that nothing is shown as sent that was
   * not: a refusal marks them Not sent, with a Retry, and leaves the card for another try, for N or for H.
   * Where they went is the thread's own row, drawn when Fleet's record of the delivery reaches `helm`.
   */
  function place(one: Thought & { stage: "match" }): void {
    if (routing === undefined || delivering.current) return;
    delivering.current = true;
    const go = () => routing.send(one.match, one.text, one.carry);
    void (one.held === undefined ? go() : one.held.deliver(go))
      .catch(() => false)
      .then((delivered) => {
        delivering.current = false;
        if (delivered && held.current.has(one.id)) leave(one.id);
      });
  }

  function startNew(one: Thought): void {
    discard(one);
    routing?.startNew(one.text);
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
        ? () => discard(thought)
        : key === "h"
          ? () => askHelm(thought)
          : thought.stage === "match"
            ? key === "enter" && !onButton
              ? () => place(thought)
              : key === "n"
                ? () => change(thought.id, restaged(thought))
                : undefined
            : key === "s"
              ? () => startNew(thought)
              : undefined;
    if (act === undefined) return;
    event.preventDefault();
    event.stopPropagation();
    act();
  }

  return (
    <div className="armada-helm-dock" data-hosted={hosted === undefined ? undefined : ""} onKeyDownCapture={keyed}>
      {hosted?.holds}
      {hosted === undefined ? (
        <HelmThread rows={[]} notice={notice} />
      ) : (
        <>
          {notice === undefined ? null : <HelmThread rows={[]} notice={notice} />}
          <ProseRefs.Provider value={refs ?? null}>{hosted.thread}</ProseRefs.Provider>
        </>
      )}
      {/* Between the thread and the message box: the reply is held open above
          it and the person is typing below it, so the ask is in the one place
          neither of them has to be left to find it — #1519. The same
          `DockQuestions` the dock draws, so there is one card and not two. */}
      <DockQuestions questions={asks} />
      {thought === undefined || thought.stage === "routing" ? null : thought.stage === "match" ? (
        <ThoughtCard
          key={thought.id}
          stage="match"
          words={thought.text}
          target={thought.match}
          onSend={() => place(thought)}
          onNo={() => change(thought.id, restaged(thought))}
          onAskHelm={() => askHelm(thought)}
        />
      ) : (
        <ThoughtCard
          key={thought.id}
          stage="new"
          words={thought.text}
          onSession={() => startNew(thought)}
          onAskHelm={() => askHelm(thought)}
          onDismiss={() => discard(thought)}
        />
      )}
      <HelmComposer
        current={current}
        repositories={options}
        recent={recent}
        location={locationOf(context, studio)}
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

/** A thought offered a new home, with its words and what went with them kept. */
function restaged(one: Thought): Thought {
  return { id: one.id, text: one.text, carry: one.carry, ...(one.held === undefined ? {} : { held: one.held }), stage: "new" };
}

/** Whether the message box holds anything: from then on a letter is a word and not an answer to the card. */
function filled(dock: HTMLElement): boolean {
  return (dock.querySelector('[role="textbox"]')?.textContent ?? "") !== "";
}

/** What went with a message, or nothing where it went with none. */
function onlyIfCarried(carry: ThoughtCarry): ThoughtCarry | undefined {
  return carry.files.length + carry.tags.length + carry.sketches.length === 0 ? undefined : carry;
}

/** The record sheet's own state, held for as long as it is open and no longer. */
type Read = { open: boolean; record?: HelmDebugInfo; failed?: string };

/**
 * Why the record could not be read, in one sentence. **Fleet's own refusal
 * where it gave one**, and the connection's where the request never left.
 */
function whyRecord(read: Extract<HelmDebugRead, { ok: false }>): string {
  const outcome = read.outcome;
  if (outcome.ok) return "The job could not be read.";
  if (outcome.why === "not_connected") return "Fleet is not connected, so the job cannot be read.";
  if (outcome.why === "refused") return outcome.error.message;
  return "The job could not be read.";
}
