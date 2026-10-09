// The Dashboard's dispatch bar: one slim line over every tab. `n` and ⌘N bring the cursor here from
// anywhere in Bridge. A Job or a Session is chosen beside the field, and Tab switches between them.
// A Job's first words hand over to the composer, which opens holding them and grows in
// (`.armada-screen__pane--grown`); a Session's words stay here and Enter starts it.

import { useEffect, useRef, type KeyboardEvent } from "react";
import { Send } from "lucide-react";
import { Kbd, REQUEST_PLACEHOLDER, Tabs, Tooltip, useKept } from "@armada/components";
import { holdsText } from "@armada/screens/src/keys";

import { useSessionsDraft } from "./sessions-draft";
import { useDispatchKind } from "./remembered-views";

const KINDS = [
  { id: "job", label: "Job" },
  { id: "session", label: "Session" },
];

/** Set when the bar was asked for before it was on screen; the bar takes it on mounting. */
let wanted = false;
/** False while Fleet cannot be reached and the bar is not drawn, so `n` keeps its older meaning. */
let drawn = true;

const FIELD = "armada-dispatch-bar__field";

/** The Dashboard says whether it draws the bar; where it does not, `n` keeps the Board's older meaning. */
export function useDispatchBarDrawn(isDrawn: boolean): void {
  useEffect(() => {
    drawn = isDrawn;
    return () => void (drawn = true);
  }, [isDrawn]);
}

/** Puts the cursor in the bar, now if it is drawn and on its mounting if it is about to be. */
export function focusDispatchBar(): void {
  const field = document.querySelector<HTMLInputElement>(`.${FIELD}`);
  if (field !== null) {
    field.focus();
    field.select();
    return;
  }
  wanted = true;
  window.setTimeout(() => (wanted = false), 1_000);
}

/**
 * `n` and ⌘N focus the bar from every surface, `goTo` being how the window reaches the Dashboard.
 * Read in the capture phase, so the surface under it never hears the key. `n` leaves a field alone;
 * ⌘N does not. Neither acts over an open dialog, which owns the keyboard.
 */
export function useDispatchBarKeys(toDashboard: () => void): void {
  const go = useRef(toDashboard);
  go.current = toDashboard;
  useEffect(() => {
    function press(event: KeyboardEvent | globalThis.KeyboardEvent): void {
      // A capital N is a Studio's Note, so a bare press must be the lowercase n.
      if ((event.metaKey ? event.key.toLowerCase() : event.key) !== "n" || event.repeat || event.defaultPrevented || event.altKey || event.ctrlKey || event.shiftKey) return;
      const chord = event.metaKey;
      if (!drawn || (!chord && holdsText(event.target)) || document.querySelector('[role="dialog"]') !== null) return;
      event.preventDefault();
      event.stopPropagation();
      go.current();
      focusDispatchBar();
    }
    window.addEventListener("keydown", press, true);
    return () => window.removeEventListener("keydown", press, true);
  }, []);
}

export function QuickDispatch({
  onType,
  focused,
  onOpenSession,
}: {
  /** A Job's first words, handed to the composer. */
  onType: (words: string) => void;
  focused: boolean;
  /** A Session just started. */
  onOpenSession: (sessionId: string) => void;
}) {
  const draft = useSessionsDraft();
  const [chosen, choose] = useDispatchKind();
  const kind = draft === undefined ? "job" : chosen;
  const [words, setWords, clearWords] = useKept("dashboard:dispatch-session", "");
  const field = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!wanted) return;
    wanted = false;
    field.current?.focus();
  }, []);

  const other = () => choose(kind === "job" ? "session" : "job");
  const onKey = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Tab" && draft !== undefined && !event.altKey && !event.ctrlKey && !event.metaKey) {
      event.preventDefault();
      other();
    } else if (event.key === "Escape") event.currentTarget.blur();
    else if (event.key === "Enter" && kind === "session" && draft !== undefined && words.trim() !== "") {
      event.preventDefault();
      const text = words.trim();
      void Promise.resolve(draft.start()).then((id) => {
        if (id === undefined) return;
        draft.send(id, { text, files: [], sketches: [], tags: [] });
        clearWords();
        onOpenSession(id);
      });
    }
  };

  return (
    <label className="armada-dispatch-bar" data-kind={kind}>
      <span className="armada-dispatch-bar__prompt" aria-hidden="true">
        <Send size={20} />
      </span>
      <input
        ref={field}
        aria-label="Request"
        className={FIELD}
        autoFocus={focused}
        value={kind === "job" ? "" : words}
        {...(kind === "job" ? { placeholder: REQUEST_PLACEHOLDER } : {})}
        onKeyDown={onKey}
        onChange={(event) => (kind === "job" ? event.target.value !== "" && onType(event.target.value) : setWords(event.target.value))}
      />
      {draft === undefined ? null : (
        <span className="armada-dispatch-bar__kind">
          <Tabs items={KINDS} value={kind} onChange={(id) => choose(id === "session" ? "session" : "job")} />
          <Tooltip label="Switch between Job and Session">
            <Kbd className="armada-dispatch-bar__tab">⇥</Kbd>
          </Tooltip>
        </span>
      )}
      <Tooltip label="Dispatch" shortcut="N">
        <Kbd>N</Kbd>
      </Tooltip>
    </label>
  );
}
