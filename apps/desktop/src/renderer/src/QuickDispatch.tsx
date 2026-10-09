// Dispatch, as the window holds it: the Dashboard's bar that grows into the dispatch panel where it
// stands, and the same panel floating over any other screen. `n` and ⌘N put the cursor in the bar
// where the Dashboard shows one, and float the panel everywhere else. A Job or a Session is chosen
// beside the field, and Tab switches between them. A Job's words, repository and settings are kept
// until it is sent, so leaving the panel (Esc, a press outside) loses nothing; a Session's words stay
// in the line and Enter starts it.

import { useCallback, useEffect, useRef, useState, type KeyboardEvent } from "react";
import type { RepositorySummary } from "@armada/protocol";
import { Kbd, REQUEST_PLACEHOLDER, Tabs, Tooltip, useKept, type DispatchSettingsValue } from "@armada/components";
import { holdsText } from "@armada/screens/src/keys";

import type { BridgeState } from "../../shared/bridge";
import { DispatchPanel, type PanelReading } from "./dispatch-panel/DispatchPanel";
import { optionsFor, useReading } from "./dispatch-panel/reading";
import { useSessionsDraft } from "./sessions-draft";
import { useDispatchKind } from "./remembered-views";

const KINDS = [
  { id: "job", label: "Job" },
  { id: "session", label: "Session" },
];

/** False while Fleet cannot be reached and the bar is not drawn, so `n` keeps its older meaning. */
let drawn = true;

const FIELD = "armada-dispatch-panel__field";

/** What asks for the floating panel: the one `DispatchFloat` mounted. */
const floaters = new Set<() => void>();

/** The Dashboard says whether it draws the bar; where it does not, `n` keeps the Board's older meaning. */
export function useDispatchBarDrawn(isDrawn: boolean): void {
  useEffect(() => {
    drawn = isDrawn;
    return () => void (drawn = true);
  }, [isDrawn]);
}

/** Puts the cursor in the bar, at the end of a draft it holds. False where no bar is on screen. */
export function focusDispatchBar(): boolean {
  const field = document.querySelector<HTMLTextAreaElement>(`.${FIELD}[data-layout="docked"]`);
  if (field === null) return false;
  field.focus();
  field.setSelectionRange(field.value.length, field.value.length);
  // A press on a bar holding a draft opens it onto the draft.
  field.click();
  return true;
}

/**
 * `n` and ⌘N bring the cursor to the bar where the Dashboard shows one, and float the panel over
 * whatever else is showing. Read in the capture phase, so the surface under it never hears the key.
 * `n` leaves a field alone; ⌘N does not. Neither acts over an open dialog, which owns the keyboard.
 */
export function useDispatchBarKeys(): void {
  useEffect(() => {
    function press(event: KeyboardEvent | globalThis.KeyboardEvent): void {
      // A capital N is a Studio's Note, so a bare press must be the lowercase n.
      if ((event.metaKey ? event.key.toLowerCase() : event.key) !== "n" || event.repeat || event.defaultPrevented || event.altKey || event.ctrlKey || event.shiftKey) return;
      const chord = event.metaKey;
      if (!drawn || (!chord && holdsText(event.target)) || document.querySelector('[role="dialog"]') !== null) return;
      event.preventDefault();
      event.stopPropagation();
      if (!focusDispatchBar()) floaters.forEach((show) => show());
    }
    window.addEventListener("keydown", press, true);
    return () => window.removeEventListener("keydown", press, true);
  }, []);
}

type Held = {
  state: BridgeState;
  repositories: readonly RepositorySummary[];
  /** A Job's request and the repository it is for, sent. */
  onDispatch: (words: string, root: string) => void;
  /** A Session just started. */
  onOpenSession: (sessionId: string) => void;
};

export function QuickDispatch({
  state,
  repositories,
  focused,
  layout = "docked",
  onClose,
  onDispatch,
  onOpenSession,
}: Held & {
  focused: boolean;
  layout?: "docked" | "floating";
  /** The floating panel was put away. */
  onClose?: () => void;
}) {
  const draft = useSessionsDraft();
  const [chosen, choose] = useDispatchKind();
  const job = draft === undefined || chosen === "job";
  const floating = layout === "floating";

  const [sessionWords, setSessionWords, clearSessionWords] = useKept("dashboard:dispatch-session", "");
  const [jobWords, setJobWords, clearJobWords] = useKept("dispatch:job-request", "");
  const [picked, pick, clearPicked] = useKept<string | null>("dispatch:job-repository", null);
  const [settings, setSettings, clearSettings] = useKept<DispatchSettingsValue>("dispatch:job-settings", {});
  const [grown, setGrown] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [asking, setAsking] = useState<"send" | "chip" | false>(false);
  const [sending, setSending] = useState(false);
  const field = useRef<HTMLTextAreaElement>(null);

  const open = job && grown;
  const options = optionsFor(jobWords, repositories, state.jobs);
  const read = useReading(jobWords, options.filter((one) => one.named).map((one) => one.root));
  const by = (root: string | null | undefined) => options.find((one) => one.root === root);
  const root = by(picked)?.root ?? by(state.repository)?.root ?? (read.state === "detected" ? read.root : undefined);
  const reading: PanelReading =
    by(picked) !== undefined
      ? { state: "picked", name: by(picked)!.name }
      : by(state.repository) !== undefined
        ? { state: "scoped", name: by(state.repository)!.name }
        : read.state === "detected"
          ? { state: "detected", name: by(read.root)!.name }
          : read;

  // Floating, the panel is the line first and grows a frame later, so it comes in the way the bar grows.
  useEffect(() => {
    if (!floating) return;
    const end = field.current?.value.length ?? 0;
    field.current?.focus();
    field.current?.setSelectionRange(end, end);
    const frame = requestAnimationFrame(() => setGrown(true));
    return () => cancelAnimationFrame(frame);
  }, [floating]);

  /** Sent: everything the panel held is let go, and the bar is a bar again. */
  function dispatch(to: string) {
    onDispatch(jobWords.trim(), to);
    clearJobWords();
    clearPicked();
    clearSettings();
    setAsking(false);
    setSending(false);
    setGrown(false);
    setSettingsOpen(false);
    if (floating) onClose?.();
    else field.current?.focus();
  }

  /** Dispatch builds nothing without a repository: a reading still out is waited for, and none found asks. */
  function send() {
    if (jobWords.trim() === "") return;
    if (root !== undefined) dispatch(root);
    else if (read.state === "detecting") setSending(true);
    else setAsking("send");
  }

  useEffect(() => {
    if (!sending || read.state === "detecting") return;
    setSending(false);
    if (jobWords.trim() === "") return;
    if (root !== undefined) dispatch(root);
    else setAsking("send");
    // The reading settling is the only thing this waits on.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sending, read.state]);

  const collapse = useCallback(() => {
    if (floating) onClose?.();
    else {
      setGrown(false);
      setAsking(false);
    }
  }, [floating, onClose]);

  const words = job ? jobWords : sessionWords;
  const other = () => choose(job ? "session" : "job");
  const onKey = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Tab" && draft !== undefined && !event.altKey && !event.ctrlKey && !event.metaKey) {
      event.preventDefault();
      other();
    } else if (event.key === "Escape") {
      if (!open) (floating ? onClose?.() : event.currentTarget.blur());
    } else if (event.key === "Enter" && !job && draft !== undefined && !event.shiftKey && words.trim() !== "") {
      event.preventDefault();
      const text = words.trim();
      void Promise.resolve(draft.start()).then((id) => {
        if (id === undefined) return;
        draft.send(id, { text, files: [], sketches: [], tags: [] });
        clearSessionWords();
        if (floating) onClose?.();
        onOpenSession(id);
      });
    }
  };

  return (
    <DispatchPanel
      layout={layout}
      open={open}
      words={words}
      onWords={(next) => {
        if (!job) return setSessionWords(next);
        setJobWords(next);
        setGrown(true);
        setAsking(false);
      }}
      placeholder={REQUEST_PLACEHOLDER}
      reading={reading}
      options={options}
      asking={asking !== false}
      onAsking={(next) => setAsking(next ? "chip" : false)}
      onPick={(chosenRoot) => {
        if (asking === "send") return dispatch(chosenRoot);
        pick(chosenRoot);
        setAsking(false);
      }}
      settings={settings}
      onSettings={setSettings}
      workflows={state.holds.workflows}
      leftOut={state.holds.leftOut}
      models={state.holds.models?.models ?? []}
      machineCap={state.limits?.concurrency ?? null}
      settingsOpen={settingsOpen}
      onSettingsOpen={setSettingsOpen}
      sending={sending}
      onSend={send}
      onOpen={() => job && jobWords !== "" && setGrown(true)}
      onCollapse={collapse}
      onDismiss={onClose}
      onFieldKeyDown={onKey}
      fieldRef={field}
      autoFocus={focused}
      disabled={state.connection.state !== "connected"}
      lead={
        draft === undefined ? null : (
          <span className="armada-dispatch-panel__kind">
            <Tabs items={KINDS} value={job ? "job" : "session"} onChange={(id) => choose(id === "session" ? "session" : "job")} />
            <Tooltip label="Switch between Job and Session">
              <Kbd className="armada-dispatch-panel__tab">⇥</Kbd>
            </Tooltip>
          </span>
        )
      }
    />
  );
}

/** The panel over whatever screen is showing, opened by `n` and ⌘N where the Dashboard's bar is not. */
export function DispatchFloat(props: Held) {
  const [shown, show] = useState(false);
  useEffect(() => {
    const open = () => show(true);
    floaters.add(open);
    return () => void floaters.delete(open);
  }, []);
  return shown ? <QuickDispatch {...props} focused={false} layout="floating" onClose={() => show(false)} /> : null;
}
