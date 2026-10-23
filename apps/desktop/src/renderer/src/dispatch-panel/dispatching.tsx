// Dispatch, as the window holds it: one draft, one entry. `n`, the Cockpit's Dispatch tile, every New session and
// a dispatch row chosen in the palette open the one sheet over the window. The entry reads this and owns no
// word of it, so leaving it (Esc, a press outside) loses nothing. A Session and an Autopilot are the kinds:
//
//   Session    needs a repository: read from the words by Fleet, else the row of chips takes the keys. Starts and opens the Session, on the model and effort
//              picked beside the act. A brief that tags a Session with `@` waits on it (the proposer reads when, from where and what is needed
//              into the sentence over the request): it holds its words and starts when the condition is met. Queue for later (⌘⇧↵) saves the idea in the Cockpit's Work list instead and opens nothing.
//   Autopilot  needs a repository, as a Session does. The act reads the words as settings, shown in the
//              panel's body for the person to edit; the act again starts it. Nothing leaves the panel.
//
// An Autopilot sent leaves a strip over the title bar's field (`DispatchedStrip`), holding the latest
// send until the next dispatch or a dismiss; ↵ or a press opens it. A Session opens itself and leaves none.
//
// **The words are read here at the moment of an act and never subscribed to**: this runs in `App`, and a key typed
// would draw the whole window. What depends on them, Fleet's reading and what it picks, is `usePanel`, in the sheet.
import { createContext, useContext, useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import type { RepositorySummary } from "@armada/protocol";
import { forgetKept, isPressed, readKept, useKept, writeKept, type ComposerTag, type DispatchKind } from "@armada/components";
import { jobIdOf, repositoryEntries, useRecentRepositories } from "@armada/shell";
import { said } from "@armada/screens/src/copy";
import { planPartsOf } from "@armada/screens/src/session-waits";

import type { BridgeState } from "../../../shared/bridge";
import { localNow, readingOf, settingsFrom } from "../cockpit/autopilot";
import { searchFiles } from "../commands";
import { useNumbered } from "../numbered";
import { useSessions, useSessionsDraft } from "../sessions-draft";
import { AutopilotBody, type AutopilotRead } from "./AutopilotBody";
import type { SentView } from "./DispatchedStrip";
import type { PanelProps, PanelReading } from "./DispatchPanel";
import { chipOf, chosen as waitChosen, handled, retagged, type Hand } from "./waits";
import { chipsFor, optionsFor, useReading } from "./reading";

/** What the request field asks for. */
const REQUEST_PLACEHOLDER = "Describe the work, or paste a link to a ticket.";

const REQUEST = "dispatch:request";
const TUNING = "dispatch:session-tuning";
const AFTER = "dispatch:session-after";

/** The model and effort picked for a Session. Absent is Auto. */
type Tuning = { model?: string; effort?: string };

/** The draft as it stands, for an act. */
const wordsNow = () => readKept<string>(REQUEST) ?? "";
const tuningNow = () => readKept<Tuning>(TUNING) ?? {};
const handNow = () => readKept<Hand>(AFTER) ?? {};

/** `queue_session`'s key in the request, rebindable: ⇧⌘Enter, with Control answering as ⌘ does everywhere in Bridge. */
const queuesOn = (event: KeyboardEvent<HTMLElement>) => isPressed("queue_session", event);

type Host = {
  state: BridgeState;
  repositories: readonly RepositorySummary[];
  openSession: (sessionId: string) => void;
  /** The repository's Autopilot lane on the Cockpit. */
  openAutopilot: (root: string) => void;
};

/** The latest Autopilot sent. */
type Sent = {
  title: string;
  /** The repository it runs in. */
  root: string;
  /** Tells this send from the one after it. */
  token: number;
};

/** What the panel is handed by an entry, short of where the entry puts it. */
export type EntryProps = Omit<PanelProps, "onDismiss" | "fieldRef" | "autoFocus">;

export type Dispatching = {
  kind: DispatchKind;
  /** The kinds Tab steps through, in order. */
  kinds: readonly DispatchKind[];
  choose: (kind: DispatchKind) => void;
  /** The sheet is up. */
  sheet: boolean;
  /**
   * A dispatch row chosen in the palette, a thought routed, a pull request's fix: the sheet holding these words, for the repository `root` where one is named.
   * `tags` are held as a `#` chosen in the request would be, so each is sent with the words while its title is still in them.
   */
  openSheet: (words: string, root?: string, tags?: readonly ComposerTag[]) => void;
  /** Every New session and compose entry: the panel on the draft as it stands, or an empty one for the repository `root` where one is named. */
  openNew: (root?: string) => void;
  /** ⌥A: the panel on a Session, with an `@` typed at the end of the request and its menu open. Absent where a Session cannot wait. */
  openAfter?: () => void;
  closeSheet: () => void;
  /** A hook, called by the sheet and nowhere else: it holds the words and their reading, which the rest of the window does not. */
  usePanel: () => EntryProps;
  /** The latest send, as the strip draws it. */
  sent: SentView | undefined;
  openSent: () => void;
  dismissSent: () => void;
  /** An Autopilot sent was opened from its strip: the Cockpit puts its bar up once, then lets this go. */
  autopilotAsked: boolean;
  autopilotShown: () => void;
};

const Held = createContext<Dispatching | undefined>(undefined);

export function DispatchingProvider({ value, children }: { value: Dispatching; children: ReactNode }) {
  return <Held.Provider value={value}>{children}</Held.Provider>;
}

/** The draft and its acts, or `undefined` where no window holds one. */
export function useDispatching(): Dispatching | undefined {
  return useContext(Held);
}

export function useDispatch({ state, repositories, openSession, openAutopilot }: Host): Dispatching {
  const draft = useSessionsDraft();
  const [recent, recall] = useRecentRepositories();
  // Autopilot is chosen for this opening only and never remembered: the next dispatch is for a Session again.
  const [autopilot, setAutopilot] = useState(false);
  const served = typeof window.armada?.proposeAutopilot === "function";
  const kinds: DispatchKind[] = [...(draft === undefined ? [] : ["session" as const]), ...(served ? ["autopilot" as const] : [])];
  const kind: DispatchKind = autopilot && served ? "autopilot" : "session";

  const [picked, pick, clearPicked] = useKept<string | null>("dispatch:repository", null);
  const [sheet, setSheet] = useState(false);
  const [autopilotAsked, setAutopilotAsked] = useState(false);
  // ⌥A pressed outside the sheet: the request is given its `@` and the menu once it is drawn.
  const [afterAsked, setAfterAsked] = useState(false);
  const [sheetGrown, setSheetGrown] = useState(false);
  const [asking, setAsking] = useState<"send" | "queue" | "chip" | false>(false);
  // The act is waiting on the repository being read, and then on Fleet's answer to it.
  const [waiting, setWaiting] = useState<false | "start" | "queue">(false);
  const [sending, setSending] = useState(false);
  const [read, setRead] = useState<AutopilotRead>();
  const [refused, setRefused] = useState<string>();
  const [latest, setLatest] = useState<Sent>();
  const sends = useRef(0);
  // The issues and pull requests the request tagged with `#`, sent with it while their words are still there.
  const tagged = useRef<ComposerTag[]>([]);
  // Bumped whenever the words change, so an answer to words no longer there is let go.
  const stamp = useRef(0);

  // The sheet is the line first and grows a frame later, so its drawer opens under the request.
  useEffect(() => {
    if (!sheet) return setSheetGrown(false);
    const frame = requestAnimationFrame(() => setSheetGrown(true));
    return () => cancelAnimationFrame(frame);
  }, [sheet]);

  /** Sent or started: everything the panel held is let go. */
  function clearAll() {
    stamp.current += 1;
    forgetKept(REQUEST);
    forgetKept(TUNING);
    forgetKept(AFTER);
    clearPicked();
    tagged.current = [];
    setAsking(false);
    setWaiting(false);
    setSending(false);
    setSheet(false);
    setRead(undefined);
    setRefused(undefined);
    setAutopilot(false);
    setLatest(undefined);
  }

  /** The strip for what was just sent. */
  function leave(send: Pick<Sent, "title" | "root">) {
    setLatest({ ...send, token: (sends.current += 1) });
  }

  const sentView: SentView | undefined = latest === undefined ? undefined : { id: latest.token, kind: "autopilot", title: latest.title };

  function openSent() {
    if (latest === undefined) return;
    openAutopilot(latest.root);
    setAutopilotAsked(true);
    setLatest(undefined);
  }

  const manifestOf = (at: string) => repositories.find((one) => one.root === at)?.manifest?.id;

  function startSession(at: string) {
    if (draft === undefined) return;
    const text = wordsNow().trim();
    recall(at);
    const hand = handNow();
    setSending(true);
    // A real reading is waited for, so a Session is not started on the defaults while the proposer is still out.
    void Promise.resolve(draft.settlesWait?.(text) ?? draft.readsWait?.(text))
      .then((read) => {
        // A brief that tags a Session that is still there waits on it, as the sentence over the request read it and as his hand left it.
        const waitsOn = read === undefined ? undefined : draft.get().find((one) => one.id === read.on.id && one.dead === undefined);
        // A tag that names no condition only refers to the Session: the new one starts at once and carries the tag.
        const refers = read?.plain === true && waitsOn !== undefined ? ({ kind: "session", id: read.on.id, title: read.on.title } as const) : undefined;
        const one = read === undefined || waitsOn === undefined || refers !== undefined ? undefined : waitChosen(read, hand, planPartsOf(waitsOn));
        const after = one === undefined ? undefined : { on: one.on, when: one.when, base: one.base, needs: one.needs, ...(one.part === undefined ? {} : { part: one.part }) };
        return Promise.resolve(after === undefined ? draft.start(undefined, at, text, tuningNow()) : draft.start(undefined, at, text, tuningNow(), after)).then((id) => ({ id, after, refers }));
      })
      .then(({ id, after, refers }) => {
        if (id === undefined) return setSending(false);
        if (after === undefined) draft.send(id, { text, files: [], sketches: [], tags: [...(refers === undefined ? [] : [refers]), ...tagged.current.filter((one) => text.includes(one.title))] });
        clearAll();
        openSession(id);
      });
  }

  /** Saves the idea in the Cockpit's Jobs and Sessions list and lets the sheet go. Nothing is started and nothing opens. */
  function queueSession(at: string) {
    if (draft?.queue === undefined) return;
    recall(at);
    draft.queue({ text: wordsNow().trim(), root: at, repository: optionsFor(repositories, []).find((one) => one.root === at)?.name ?? at, ...tuningNow() });
    clearAll();
  }

  function readAutopilot(at: string) {
    const manifest = manifestOf(at);
    if (manifest === undefined) return;
    const mine = stamp.current;
    setSending(true);
    setRefused(undefined);
    void window.armada.proposeAutopilot(manifest, wordsNow().trim(), localNow(Date.now())).then((answer) => {
      if (mine !== stamp.current) return;
      setSending(false);
      if (!answer.ok) return setRefused(said(answer.outcome));
      setRead({ instruction: answer.value.instruction, ...readingOf(answer.value) });
    });
  }

  function startAutopilot(at: string) {
    const manifest = manifestOf(at);
    if (manifest === undefined || read === undefined) return;
    const settingsRead = settingsFrom(read.instruction, read.fields, Date.now());
    if (!settingsRead.ok) return setRefused(settingsRead.why);
    const mine = stamp.current;
    setSending(true);
    setRefused(undefined);
    void window.armada.startAutopilot(manifest, settingsRead.settings).then((answer) => {
      if (mine !== stamp.current) return;
      if (!answer.ok) return (setSending(false), setRefused(said(answer.outcome)));
      recall(at);
      clearAll();
      leave({ title: optionsFor(repositories, []).find((one) => one.root === at)?.name ?? at, root: at });
    });
  }

  function proceed(at: string) {
    if (kind === "session") startSession(at);
    else if (read === undefined) readAutopilot(at);
    else startAutopilot(at);
  }

  const choose = (next: DispatchKind) => {
    setAutopilot(next === "autopilot");
  };

  const cycle = () => choose(kinds[(kinds.indexOf(kind) + 1) % kinds.length] ?? "session");

  const typed = (next: string) => {
    stamp.current += 1;
    writeKept(REQUEST, next);
    setLatest(undefined);
    setAsking(false);
    setRead(undefined);
    setRefused(undefined);
    setWaiting(false);
    setSending(false);
  };

  const body: ReactNode = kind === "autopilot" && (read !== undefined || refused !== undefined) ? <AutopilotBody read={read} onRead={setRead} refused={refused} /> : undefined;

  const collapse = () => {
    setAsking(false);
    setSheet(false);
  };

  const usePanel = (): EntryProps => {
    const [words] = useKept(REQUEST, "");
    const [tuned, setTuned] = useKept<Tuning>(TUNING, {});
    const [hand, setHand] = useKept<Hand>(AFTER, {});
    const live = useSessions();
    const offered = live.filter((one) => one.dead === undefined && one.helm !== true && one.title !== undefined).map((one) => ({ id: one.id, title: one.title! }));
    const { reading: found, likely } = useReading(words);
    const options = optionsFor(repositories, likely);
    const by = (root: string | null | undefined) => options.find((one) => one.root === root);
    // Fleet names the repository by its Manifest's id, and the panel works in roots.
    const detected = found.state === "detected" ? by(repositories.find((one) => jobIdOf(one) === found.manifestId)?.root) : undefined;
    const root = by(picked)?.root ?? by(state.repository)?.root ?? detected?.root;
    // Files and numbered work are a repository's, so they are offered once the request is for one.
    const named = repositories.find((one) => one.root === root);
    const manifestId = named === undefined ? undefined : jobIdOf(named);
    const numbered = useNumbered(manifestId);
    const reading: PanelReading =
      by(picked) !== undefined
        ? { state: "picked", name: by(picked)!.name }
        : by(state.repository) !== undefined
          ? { state: "scoped", name: by(state.repository)!.name }
          : detected !== undefined
            ? { state: "detected", name: detected.name }
            : found.state === "detected"
              ? { state: "unresolved" }
              : found;

    // The sentence over the request: what the proposer read of the words, and his hand over it. A new Session picked on it is written into the words.
    // Not `read`, which is the Autopilot's reading of the words and names the act below.
    const proposed = kind === "session" ? draft?.readsWait?.(words) : undefined;
    const sentence = proposed?.plain === true ? undefined : proposed;
    const wait =
      sentence === undefined
        ? undefined
        : chipOf(sentence, hand, planPartsOf(live.find((one) => one.id === sentence.on.id)), (part, value) => {
            if (part === "on") {
              const next = offered.find((one) => one.id === value);
              if (next !== undefined) typed(retagged(words, sentence.on.title, next.title));
            }
            setHand(handled(sentence, hand, part, value));
          });

    /** Nothing starts without a repository: a reading still out is waited for, and none found asks. */
    const act = () => {
      if (words.trim() === "") return;
      if (root === undefined) return found.state === "detecting" ? setWaiting("start") : setAsking("send");
      proceed(root);
    };

    /** Queue for later holds to the same rule: no repository, and the row asks before the idea is saved. */
    const queue = () => {
      if (words.trim() === "") return;
      if (root === undefined) return found.state === "detecting" ? setWaiting("queue") : setAsking("queue");
      queueSession(root);
    };

    // A reading belongs to the repository it was made for.
    useEffect(() => setRead(undefined), [root]);

    useEffect(() => {
      if (!waiting || (root === undefined && found.state === "detecting")) return;
      setWaiting(false);
      if (waiting === "queue") queue();
      else act();
      // The reading settling, or a repository picked before it does, is all this waits on.
      // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [waiting, found.state, root]);

    return {
      open: sheetGrown,
      words,
      onWords: typed,
      placeholder: REQUEST_PLACEHOLDER,
      kind,
      kinds,
      onKind: choose,
      reading,
      chips: chipsFor(options, { scoped: by(state.repository)?.root, read: detected?.root, chosen: root, recent }),
      entries: repositoryEntries(repositories, { scope: root, recent }),
      asking: asking !== false,
      onAsking: (next) => setAsking(next ? "chip" : false),
      onPick: (at) => {
        if (asking === "send" && kind === "session") return (setAsking(false), startSession(at));
        if (asking === "queue") return (setAsking(false), queueSession(at));
        pick(at);
        if (asking === "send") (setAsking(false), readAutopilot(at));
        else setAsking(false);
      },
      act: kind === "session" ? "Start" : read === undefined ? "Read it" : "Start Autopilot",
      sending: sending || waiting === "start",
      onAct: act,
      ...(kind === "session" && draft !== undefined
        ? {
            tuning: {
              models: draft.models,
              ...(draft.defaultModel === undefined ? {} : { defaultModel: draft.defaultModel }),
              efforts: draft.efforts,
              model: tuned.model ?? null,
              effort: tuned.effort ?? null,
              onTune: (next: { model: string | null; effort: string | null }) =>
                setTuned({ ...(next.model === null ? {} : { model: next.model }), ...(next.effort === null ? {} : { effort: next.effort }) }),
            },
            ...(draft.queue === undefined ? {} : { onQueue: queue, queueing: waiting === "queue" }),
          }
        : {}),
      // Offered to a Session's request, once the repository is known.
      ...(manifestId === undefined
        ? {}
        : { tags: { searchFiles: (query: string) => searchFiles(query, manifestId), ...(numbered === undefined ? {} : { numbered }), onTag: (tag: ComposerTag) => void tagged.current.push(tag) } }),
      // Offered from any kind, since ⌥A on an Autopilot's request turns it into a Session's.
      ...(draft?.readsWait === undefined ? {} : { wait: { sessions: offered, ...(wait === undefined ? {} : { chip: wait }) }, focusAfter: afterAsked, onAfterFocused: () => setAfterAsked(false) }),
      onCollapse: collapse,
      onFieldKeyDown: (event: KeyboardEvent<HTMLTextAreaElement>) => {
        if (queuesOn(event) && kind === "session" && draft?.queue !== undefined) {
          event.preventDefault();
          if (words.trim() !== "") queue();
        } else if (event.key === "Tab" && kinds.length > 1 && !event.altKey && !event.ctrlKey && !event.metaKey) {
          event.preventDefault();
          cycle();
        } else if (event.key === "Escape") {
          if (!sheetGrown) collapse();
        } else if (event.key === "Enter" && kind === "session" && !event.shiftKey && words.trim() !== "") {
          event.preventDefault();
          act();
        }
      },
      ...(body === undefined ? {} : { body }),
      disabled: state.connection.state !== "connected",
    };
  };

  const openNew = (at?: string) => {
    if (at === undefined && wordsNow().trim() !== "") return setSheet(true);
    stamp.current += 1;
    writeKept(REQUEST, "");
    if (at !== undefined) pick(at);
    setLatest(undefined);
    setAsking(false);
    setRead(undefined);
    setRefused(undefined);
    setSheet(true);
  };

  /** ⌥A from anywhere: the draft as it stands with an `@` after it, on a Session, and the sheet up. */
  function openAfter() {
    const was = wordsNow();
    choose("session");
    stamp.current += 1;
    writeKept(REQUEST, was.trim() === "" ? "@" : /\s$/.test(was) ? `${was}@` : `${was} @`);
    setLatest(undefined);
    setAsking(false);
    setRead(undefined);
    setRefused(undefined);
    setAfterAsked(true);
    setSheet(true);
  }

  return {
    kind,
    kinds,
    choose,
    sheet,
    openNew,
    ...(draft?.readsWait === undefined || !kinds.includes("session") ? {} : { openAfter }),
    openSheet: (next, at, tags) => {
      stamp.current += 1;
      writeKept(REQUEST, next);
      if (tags !== undefined) tagged.current = [...tags];
      if (at !== undefined) pick(at);
      setLatest(undefined);
      setAsking(false);
      setRead(undefined);
      setRefused(undefined);
      setSheet(true);
    },
    closeSheet: collapse,
    usePanel,
    sent: sentView,
    openSent,
    dismissSent: () => setLatest(undefined),
    autopilotAsked,
    autopilotShown: () => setAutopilotAsked(false),
  };
}
