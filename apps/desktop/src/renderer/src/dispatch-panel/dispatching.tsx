// Dispatch, as the window holds it: one draft, one entry. `n`, the Cockpit's Dispatch tile, every New job and
// a dispatch row chosen in the palette open the one sheet over the window. The entry reads this and owns no
// word of it, so leaving it (Esc, a press outside) loses nothing. A Job, a Session and an Autopilot are the kinds:
//
//   Job        needs a repository: read from the words by Fleet, else the row of chips takes the keys. Sends `proposeFrom`.
//   Session    needs a repository, as a Job does (the owner, 13 Oct 2026). Starts and opens the Session, on the model and effort
//              picked beside the act. Picked to wait on another Session (the After row), it holds its words and starts when the condition is met. Queue for later (⌘⇧↵) saves the idea in the Cockpit's Jobs and Sessions list instead and opens nothing.
//   Autopilot  needs a repository, as a Job does. The act reads the words as settings, shown in the
//              panel's body for the person to edit; the act again starts it. Nothing leaves the panel.
//
// A Job or an Autopilot sent leaves a strip over the title bar's field (`DispatchedStrip`), holding the latest
// send until the next dispatch or a dismiss; ↵ or a press opens it. A Session opens itself and leaves none.
//
// **The words are read here at the moment of an act and never subscribed to**: this runs in `App`, and a key typed
// would draw the whole window. What depends on them, Fleet's reading and what it picks, is `usePanel`, in the sheet.
import { createContext, useContext, useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import type { RepositorySummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";
import { DispatchSettings, REQUEST_PLACEHOLDER, forgetKept, isPressed, readKept, useKept, writeKept, type DispatchKind, type DispatchSettingsValue } from "@armada/components";
import { repositoryEntries, useRecentRepositories } from "@armada/shell";
import { said } from "@armada/screens/src/copy";
import { planPartsOf } from "@armada/screens/src/session-waits";

import type { BridgeState } from "../../../shared/bridge";
import { localNow, readingOf, settingsFrom } from "../cockpit/autopilot";
import { useDispatchKind } from "../remembered-views";
import { useSessions, useSessionsDraft } from "../sessions-draft";
import { AutopilotBody, type AutopilotRead } from "./AutopilotBody";
import type { SentView } from "./DispatchedStrip";
import type { AfterPick, PanelProps, PanelReading } from "./DispatchPanel";
import { chipsFor, optionsFor, useReading } from "./reading";

const REQUEST = "dispatch:request";
const TUNING = "dispatch:session-tuning";
const AFTER = "dispatch:session-after";

/** The model and effort picked for a Session. Absent is Auto. */
type Tuning = { model?: string; effort?: string };

/** The draft as it stands, for an act. */
const wordsNow = () => readKept<string>(REQUEST) ?? "";
const tuningNow = () => readKept<Tuning>(TUNING) ?? {};
const afterNow = () => readKept<AfterPick | null>(AFTER) ?? undefined;

/** `queue_session`'s key in the request, rebindable: ⇧⌘Enter, with Control answering as ⌘ does everywhere in Bridge. */
const queuesOn = (event: KeyboardEvent<HTMLElement>) => isPressed("queue_session", event);

type Host = {
  state: BridgeState;
  repositories: readonly RepositorySummary[];
  /** A Job's request and the repository it is for. Resolves to what Fleet refused it with, if it did. */
  proposeFrom: (request: string, attachments: readonly [], repository: string | null) => Promise<string | null>;
  openSession: (sessionId: string) => void;
  /** The Job's detail. */
  openJob: (jobId: string) => void;
  /** The repository's Autopilot lane on the Cockpit. */
  openAutopilot: (root: string) => void;
  said: (sentence: string) => void;
};

/** The latest Job or Autopilot sent. */
type Sent = {
  kind: "job" | "autopilot";
  title: string;
  /** The Jobs on the Board before the send. The Job the send made is not among them. */
  before: ReadonlySet<string>;
  /** The Job it made, once Fleet has named it on the call. */
  jobId?: string;
  /** An Autopilot's repository. */
  root?: string;
  refused?: string;
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
  /** A dispatch row chosen in the palette, a thought routed, a pull request's fix: the sheet holding these words, for the repository `root` where one is named. */
  openSheet: (words: string, root?: string) => void;
  /** Every New job and compose entry: the panel on the draft as it stands, or an empty one for the repository `root` where one is named. */
  openNew: (root?: string) => void;
  /** ⌥A: the panel on a Session, with the focus on the Session it waits on. Absent where a Session cannot wait. */
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

/** The After row: the live Sessions a new one can wait on, and the pick if that Session is still there. Helm's own is not offered. */
function afterOf(sessions: readonly Session[], picked: AfterPick | null, set: (next: AfterPick | null) => void): NonNullable<PanelProps["after"]> {
  const offered = sessions.filter((one) => one.dead === undefined && one.helm !== true);
  const on = picked === null ? undefined : offered.find((one) => one.id === picked.on);
  const parts = planPartsOf(on);
  return {
    sessions: offered.map((one) => ({ id: one.id, title: one.title ?? one.id })),
    ...(picked === null || on === undefined ? {} : { picked: parts.length === 0 && picked.when === "plan" ? { on: picked.on, when: "ends" as const } : picked }),
    parts,
    onPick: (next) => set(next ?? null),
  };
}

/** A Job's request is a long sentence and a title is a line. */
const firstLine = (words: string) => words.trim().split("\n")[0] ?? "";

export function useDispatch({ state, repositories, proposeFrom, openSession, openJob, openAutopilot, said: tell }: Host): Dispatching {
  const draft = useSessionsDraft();
  const [recent, recall] = useRecentRepositories();
  const [chosen, remember] = useDispatchKind();
  // Autopilot is chosen for this opening only and never remembered: the next dispatch is for a Job again.
  const [autopilot, setAutopilot] = useState(false);
  const served = typeof window.armada?.proposeAutopilot === "function";
  const kinds: DispatchKind[] = ["job", ...(draft === undefined ? [] : ["session" as const]), ...(served ? ["autopilot" as const] : [])];
  const kind: DispatchKind = autopilot && served ? "autopilot" : draft === undefined ? "job" : chosen;

  const [picked, pick, clearPicked] = useKept<string | null>("dispatch:repository", null);
  const [settings, setSettings, clearSettings] = useKept<DispatchSettingsValue>("dispatch:job-settings", {});
  const [sheet, setSheet] = useState(false);
  const [autopilotAsked, setAutopilotAsked] = useState(false);
  // ⌥A pressed outside the sheet: the focus goes to its After row once it is drawn.
  const [afterAsked, setAfterAsked] = useState(false);
  const [sheetGrown, setSheetGrown] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [asking, setAsking] = useState<"send" | "queue" | "chip" | false>(false);
  // The act is waiting on the repository being read, and then on Fleet's answer to it.
  const [waiting, setWaiting] = useState<false | "start" | "queue">(false);
  const [sending, setSending] = useState(false);
  const [read, setRead] = useState<AutopilotRead>();
  const [refused, setRefused] = useState<string>();
  const [latest, setLatest] = useState<Sent>();
  const sends = useRef(0);
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
    clearSettings();
    setAsking(false);
    setWaiting(false);
    setSending(false);
    setSheet(false);
    setSettingsOpen(false);
    setRead(undefined);
    setRefused(undefined);
    setAutopilot(false);
    setLatest(undefined);
  }

  /** The strip for what was just sent. */
  function leave(send: Pick<Sent, "kind" | "title"> & Partial<Pick<Sent, "root">>): number {
    const token = (sends.current += 1);
    setLatest({ ...send, before: new Set(state.jobs.map((job) => job.id)), token });
    return token;
  }

  // Fleet names the Job on the call from its first move, well before the proposer answers. A call
  // still standing from an earlier send names a Job that was on the Board before this one.
  const proposed = state.proposing?.job_id;
  useEffect(() => {
    if (proposed === undefined) return;
    setLatest((now) => (now?.kind === "job" && now.jobId === undefined && !now.before.has(proposed) ? { ...now, jobId: proposed } : now));
  }, [proposed, latest?.token]);
  const made = latest?.kind === "job" && latest.jobId !== undefined ? state.jobs.find((job) => job.id === latest.jobId) : undefined;
  const sentView: SentView | undefined =
    latest === undefined
      ? undefined
      : {
          id: latest.token,
          kind: latest.kind,
          title: made === undefined ? latest.title : firstLine(made.title),
          ready: latest.kind === "autopilot" || latest.jobId !== undefined,
          ...(latest.refused === undefined ? {} : { refused: latest.refused }),
        };

  function openSent() {
    if (latest === undefined) return;
    if (latest.kind === "job") {
      if (latest.jobId === undefined) return;
      openJob(latest.jobId);
    } else if (latest.root !== undefined) (openAutopilot(latest.root), setAutopilotAsked(true));
    setLatest(undefined);
  }

  const manifestOf = (at: string) => repositories.find((one) => one.root === at)?.manifest?.id;

  function sendJob(at: string) {
    const words = wordsNow();
    const title = firstLine(words);
    const request = words.trim();
    recall(at);
    clearAll();
    const mine = leave({ kind: "job", title });
    // A refusal is said on the strip while it is still the latest send, and as a notice once it is not.
    void proposeFrom(request, [], at).then((told) => {
      if (told === null) return;
      if (sends.current === mine) setLatest((now) => (now?.token === mine ? { ...now, refused: told } : now));
      else tell(told);
    });
  }

  function startSession(at: string) {
    if (draft === undefined) return;
    const text = wordsNow().trim();
    recall(at);
    // Picked to wait on a Session that is still there: it holds its words, and nothing is sent until its condition is met.
    const picked = afterNow();
    const waitsOn = picked === undefined ? undefined : draft.get().find((one) => one.id === picked.on && one.dead === undefined);
    const parts = planPartsOf(waitsOn);
    const part = picked?.when !== "plan" ? undefined : (parts.find((one) => one.id === picked.part) ?? parts[0]);
    const after = waitsOn === undefined || picked === undefined ? undefined : { on: { id: waitsOn.id, title: waitsOn.title ?? waitsOn.id }, when: picked.when, ...(part === undefined ? {} : { part: { id: part.id, title: part.title } }) };
    setSending(true);
    void Promise.resolve(draft.start(undefined, at, text, tuningNow(), after)).then((id) => {
      if (id === undefined) return setSending(false);
      if (after === undefined) draft.send(id, { text, files: [], sketches: [], tags: [] });
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
      leave({ kind: "autopilot", title: optionsFor(repositories, []).find((one) => one.root === at)?.name ?? at, root: at });
    });
  }

  function proceed(at: string) {
    if (kind === "session") startSession(at);
    else if (kind === "job") sendJob(at);
    else if (read === undefined) readAutopilot(at);
    else startAutopilot(at);
  }

  const choose = (next: DispatchKind) => {
    setAutopilot(next === "autopilot");
    if (next !== "autopilot") remember(next);
  };

  const cycle = () => choose(kinds[(kinds.indexOf(kind) + 1) % kinds.length] ?? "job");

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

  const body: ReactNode =
    kind === "job" ? (
      <DispatchSettings
        open={settingsOpen}
        onOpenChange={setSettingsOpen}
        settings={settings}
        onSettings={setSettings}
        workflows={state.holds.workflows}
        {...(state.holds.leftOut === undefined ? {} : { leftOut: state.holds.leftOut })}
        models={state.holds.models?.models ?? []}
        machineCap={state.limits?.concurrency ?? null}
        disabled={state.connection.state !== "connected"}
      />
    ) : kind === "autopilot" && (read !== undefined || refused !== undefined) ? (
      <AutopilotBody read={read} onRead={setRead} refused={refused} />
    ) : undefined;

  const collapse = () => {
    setAsking(false);
    setSheet(false);
  };

  const usePanel = (): EntryProps => {
    const [words] = useKept(REQUEST, "");
    const [tuned, setTuned] = useKept<Tuning>(TUNING, {});
    const [waitsOn, setWaitsOn] = useKept<AfterPick | null>(AFTER, null);
    const live = useSessions();
    const { reading: found, likely } = useReading(words);
    const options = optionsFor(repositories, likely);
    const by = (root: string | null | undefined) => options.find((one) => one.root === root);
    // Fleet names the repository by its Manifest's id, and the panel works in roots.
    const detected = found.state === "detected" ? by(repositories.find((one) => one.manifest?.id === found.manifestId)?.root) : undefined;
    const root = by(picked)?.root ?? by(state.repository)?.root ?? detected?.root;
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
      if (!waiting || found.state === "detecting") return;
      setWaiting(false);
      if (waiting === "queue") queue();
      else act();
      // The reading settling is the only thing this waits on.
      // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [waiting, found.state]);

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
      entries: repositoryEntries(repositories, { scope: root, onlySetUp: true, recent }),
      asking: asking !== false,
      onAsking: (next) => setAsking(next ? "chip" : false),
      onPick: (at) => {
        if (asking === "send" && kind === "job") return sendJob(at);
        if (asking === "send" && kind === "session") return (setAsking(false), startSession(at));
        if (asking === "queue") return (setAsking(false), queueSession(at));
        pick(at);
        if (asking === "send") (setAsking(false), readAutopilot(at));
        else setAsking(false);
      },
      act: kind === "job" ? "Dispatch" : kind === "session" ? "Start" : read === undefined ? "Read it" : "Start Autopilot",
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
      // Offered from any kind, since ⌥A on a Job's request turns it into a Session's.
      ...(draft?.startWhen === undefined ? {} : { after: afterOf(live, waitsOn, setWaitsOn), focusAfter: afterAsked, onAfterFocused: () => setAfterAsked(false) }),
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

  return {
    kind,
    kinds,
    choose,
    sheet,
    openNew,
    ...(draft?.startWhen === undefined || !kinds.includes("session") ? {} : { openAfter: () => (choose("session"), setAfterAsked(true), openNew()) }),
    openSheet: (next, at) => {
      stamp.current += 1;
      writeKept(REQUEST, next);
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
