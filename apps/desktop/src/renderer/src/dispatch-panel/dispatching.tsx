// Dispatch, as the window holds it: one draft, two entries. The Cockpit's bar grows into the panel in
// place; the title bar's one bar, once something is chosen to dispatch, grows it down from its field.
// Both read this and neither owns a word of it, so what was typed in one is in the other, and leaving
// either (Esc, a press outside) loses nothing. A Job, a Session and an Autopilot are the kinds:
//
//   Job        needs a repository: read from the words by Fleet, else asked inline. Sends `proposeFrom`.
//   Session    needs a repository, as a Job does (the owner, 13 Oct 2026). Starts and opens the Session.
//   Autopilot  needs a repository, as a Job does. The act reads the words as settings, shown in the
//              panel's body for the person to edit; the act again starts it. Nothing leaves the panel.
//
// A Job or an Autopilot sent leaves a strip where the bar was (`DispatchedStrip`), holding the latest
// send until the next dispatch or a dismiss; ↵ or a press opens it. A Session opens itself and leaves none.
import { createContext, useContext, useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import type { RepositorySummary } from "@armada/protocol";
import { DispatchSettings, REQUEST_PLACEHOLDER, useKept, type DispatchKind, type DispatchSettingsValue } from "@armada/components";
import { said } from "@armada/screens/src/copy";

import type { BridgeState } from "../../../shared/bridge";
import { localNow, readingOf, settingsFrom } from "../cockpit/autopilot";
import { useDispatchKind } from "../remembered-views";
import { useSessionsDraft } from "../sessions-draft";
import { AutopilotBody, type AutopilotRead } from "./AutopilotBody";
import type { SentView } from "./DispatchedStrip";
import type { PanelProps, PanelReading } from "./DispatchPanel";
import { optionsFor, useReading } from "./reading";

/** Which entry is asking for the props: the Cockpit's bar, or the panel grown from the title bar. */
export type Entry = "bar" | "title";

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

/** The latest Job or Autopilot sent, and where it was sent from. */
type Sent = {
  kind: "job" | "autopilot";
  entry: Entry;
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
export type EntryProps = Omit<PanelProps, "layout" | "anchor" | "away" | "onDismiss" | "fieldRef" | "autoFocus">;

export type Dispatching = {
  kind: DispatchKind;
  /** The kinds Tab steps through, in order. */
  kinds: readonly DispatchKind[];
  choose: (kind: DispatchKind) => void;
  /** The title bar's panel is up. */
  titled: boolean;
  /** The one bar chose to dispatch: the panel grows down from its field holding these words. */
  openTitled: (words: string) => void;
  closeTitled: () => void;
  panel: (entry: Entry) => EntryProps;
  /** The latest send, as the strip draws it, and the entry it was sent from. */
  sent: (SentView & { entry: Entry }) | undefined;
  openSent: () => void;
  dismissSent: () => void;
};

const Held = createContext<Dispatching | undefined>(undefined);

export function DispatchingProvider({ value, children }: { value: Dispatching; children: ReactNode }) {
  return <Held.Provider value={value}>{children}</Held.Provider>;
}

/** The draft and its acts, or `undefined` where no window holds one. */
export function useDispatching(): Dispatching | undefined {
  return useContext(Held);
}

/** A Job's request is a long sentence and a title is a line. */
const firstLine = (words: string) => words.trim().split("\n")[0] ?? "";

export function useDispatch({ state, repositories, proposeFrom, openSession, openJob, openAutopilot, said: tell }: Host): Dispatching {
  const draft = useSessionsDraft();
  const [chosen, remember] = useDispatchKind();
  // Autopilot is chosen for this opening only and never remembered: the next dispatch is for a Job again.
  const [autopilot, setAutopilot] = useState(false);
  const served = typeof window.armada?.proposeAutopilot === "function";
  const kinds: DispatchKind[] = ["job", ...(draft === undefined ? [] : ["session" as const]), ...(served ? ["autopilot" as const] : [])];
  const kind: DispatchKind = autopilot && served ? "autopilot" : draft === undefined ? "job" : chosen;

  const [words, setWords, clearWords] = useKept("dispatch:request", "");
  const [picked, pick, clearPicked] = useKept<string | null>("dispatch:repository", null);
  const [settings, setSettings, clearSettings] = useKept<DispatchSettingsValue>("dispatch:job-settings", {});
  const [grown, setGrown] = useState(false);
  const [titled, setTitled] = useState(false);
  const [titleGrown, setTitleGrown] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [asking, setAsking] = useState<"send" | "chip" | false>(false);
  // The act is waiting on the repository being read, and then on Fleet's answer to it.
  const [waiting, setWaiting] = useState(false);
  const [sending, setSending] = useState(false);
  const [read, setRead] = useState<AutopilotRead>();
  const [refused, setRefused] = useState<string>();
  const [latest, setLatest] = useState<Sent>();
  const sends = useRef(0);
  // Bumped whenever the words change, so an answer to words no longer there is let go.
  const stamp = useRef(0);

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

  // The title bar's panel is the line first and grows a frame later, so it comes in the way the bar grows.
  useEffect(() => {
    if (!titled) return setTitleGrown(false);
    const frame = requestAnimationFrame(() => setTitleGrown(true));
    return () => cancelAnimationFrame(frame);
  }, [titled]);

  // A reading belongs to the repository it was made for.
  useEffect(() => setRead(undefined), [root]);

  /** Sent or started: everything the panel held is let go, and the bar is a bar again. */
  function clearAll() {
    stamp.current += 1;
    clearWords();
    clearPicked();
    clearSettings();
    setAsking(false);
    setWaiting(false);
    setSending(false);
    setGrown(false);
    setTitled(false);
    setSettingsOpen(false);
    setRead(undefined);
    setRefused(undefined);
    setAutopilot(false);
    setLatest(undefined);
  }

  /** The strip for what was just sent, from the entry the draft was in. */
  function leave(send: Pick<Sent, "kind" | "title"> & Partial<Pick<Sent, "root">>): number {
    const token = (sends.current += 1);
    setLatest({ ...send, entry: titled ? "title" : "bar", before: new Set(state.jobs.map((job) => job.id)), token });
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
  const sentView: (SentView & { entry: Entry }) | undefined =
    latest === undefined
      ? undefined
      : {
          id: latest.token,
          kind: latest.kind,
          entry: latest.entry,
          title: made === undefined ? latest.title : firstLine(made.title),
          ready: latest.kind === "autopilot" || latest.jobId !== undefined,
          ...(latest.refused === undefined ? {} : { refused: latest.refused }),
        };

  function openSent() {
    if (latest === undefined) return;
    if (latest.kind === "job") {
      if (latest.jobId === undefined) return;
      openJob(latest.jobId);
    } else if (latest.root !== undefined) openAutopilot(latest.root);
    setLatest(undefined);
  }

  const manifestOf = (at: string) => repositories.find((one) => one.root === at)?.manifest?.id;

  function sendJob(at: string) {
    const title = firstLine(words);
    const request = words.trim();
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
    const text = words.trim();
    setSending(true);
    void Promise.resolve(draft.start(undefined, at)).then((id) => {
      if (id === undefined) return setSending(false);
      draft.send(id, { text, files: [], sketches: [], tags: [] });
      clearAll();
      openSession(id);
    });
  }

  function readAutopilot(at: string) {
    const manifest = manifestOf(at);
    if (manifest === undefined) return;
    const mine = stamp.current;
    setSending(true);
    setRefused(undefined);
    void window.armada.proposeAutopilot(manifest, words.trim(), localNow(Date.now())).then((answer) => {
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
      clearAll();
      leave({ kind: "autopilot", title: by(at)?.name ?? at, root: at });
    });
  }

  /** Nothing starts without a repository: a reading still out is waited for, and none found asks. */
  function act() {
    if (words.trim() === "") return;
    if (root === undefined) return found.state === "detecting" ? setWaiting(true) : setAsking("send");
    proceed(root);
  }

  function proceed(at: string) {
    if (kind === "session") startSession(at);
    else if (kind === "job") sendJob(at);
    else if (read === undefined) readAutopilot(at);
    else startAutopilot(at);
  }

  useEffect(() => {
    if (!waiting || found.state === "detecting") return;
    setWaiting(false);
    act();
    // The reading settling is the only thing this waits on.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [waiting, found.state]);

  const choose = (next: DispatchKind) => {
    setAutopilot(next === "autopilot");
    if (next !== "autopilot") remember(next);
  };

  const cycle = () => choose(kinds[(kinds.indexOf(kind) + 1) % kinds.length] ?? "job");

  const typed = (entry: Entry, next: string) => {
    stamp.current += 1;
    setWords(next);
    setLatest(undefined);
    if (entry === "bar") setGrown(true);
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

  const collapse = (entry: Entry) => {
    setAsking(false);
    if (entry === "bar") setGrown(false);
    else setTitled(false);
  };

  const panel = (entry: Entry): EntryProps => ({
    open: entry === "bar" ? grown : titleGrown,
    words,
    onWords: (next) => typed(entry, next),
    placeholder: REQUEST_PLACEHOLDER,
    kind,
    kinds,
    onKind: choose,
    reading,
    options,
    asking: asking !== false,
    onAsking: (next) => setAsking(next ? "chip" : false),
    onPick: (at) => {
      if (asking === "send" && kind === "job") return sendJob(at);
      if (asking === "send" && kind === "session") return (setAsking(false), startSession(at));
      pick(at);
      if (asking === "send") (setAsking(false), readAutopilot(at));
      else setAsking(false);
    },
    act: kind === "job" ? "Dispatch" : kind === "session" ? "Start" : read === undefined ? "Read it" : "Start Autopilot",
    sending: sending || waiting,
    onAct: act,
    onOpen: () => words !== "" && setGrown(true),
    onCollapse: () => collapse(entry),
    onFieldKeyDown: (event: KeyboardEvent<HTMLTextAreaElement>) => {
      if (event.key === "Tab" && kinds.length > 1 && !event.altKey && !event.ctrlKey && !event.metaKey) {
        event.preventDefault();
        cycle();
      } else if (event.key === "Escape") {
        if (latest?.entry === entry && words === "") setLatest(undefined);
        else if (!(entry === "bar" ? grown : titleGrown)) entry === "bar" ? event.currentTarget.blur() : collapse(entry);
      } else if (event.key === "Enter" && latest?.entry === entry && words === "" && !event.shiftKey) {
        event.preventDefault();
        openSent();
      } else if (event.key === "Enter" && kind === "session" && !event.shiftKey && words.trim() !== "") {
        event.preventDefault();
        act();
      }
    },
    ...(body === undefined ? {} : { body }),
    ...(sentView?.entry === entry ? { sent: sentView, onOpenSent: openSent, onDismissSent: () => setLatest(undefined) } : {}),
    disabled: state.connection.state !== "connected",
  });

  return {
    kind,
    kinds,
    choose,
    titled,
    openTitled: (next) => {
      stamp.current += 1;
      setWords(next);
      setLatest(undefined);
      setAsking(false);
      setRead(undefined);
      setRefused(undefined);
      setTitled(true);
    },
    closeTitled: () => collapse("title"),
    panel,
    sent: sentView,
    openSent,
    dismissSent: () => setLatest(undefined),
  };
}
