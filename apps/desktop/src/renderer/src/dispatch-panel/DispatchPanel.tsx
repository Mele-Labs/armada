// The dispatch panel: the place work is described, a centred sheet over the window that `n`, the Cockpit's
// Dispatch tile, every New session and a dispatch row chosen in the palette all open. One frame holds it: the line (prompt, request, the key that switches kind)
// never moves, and a drawer under it opens for the band (kind, repository, the act) and a body that depends on the kind: an Autopilot's
// read settings, nothing for a Session. The request is one textarea that is
// never remounted, so what was typed and the caret stay where they were while the frame grows.
//
// The repository is a row of chips under the request, never a dropdown: each carries its key, ⌥1 to ⌥9, and
// the one the words name lights as they are typed. Past nine a trailing menu holds the rest. When none is
// chosen the act gives the row the keys (← and →, ↵ picks and sends) before anything is built.
// A Session's band also holds its Model and Effort, the composer's own pair of selects, and Queue for later beside Start: ⌘⇧↵ saves the idea
// without starting it. State is carried by the frame (its edge and hue) and by the chosen chip's fill, not by a phrase.
// A Session may wait on another, and the brief says so: an `@` in the request opens the live Sessions (arrows, ↵, Esc), the Session picked stands in
// the words as a tag, and a sentence over the request shows the proposer's reading of the wait, each part a button (`WaitChip`). ⌥A (`dispatch_after`)
// opens the sheet on a Session with an `@` typed.

import { useEffect, useId, useLayoutEffect, useRef, useState, type CSSProperties, type KeyboardEvent, type ReactNode, type Ref } from "react";
import { Cpu, FolderGit2, LoaderCircle, Zap } from "lucide-react";
import { Button, DropdownMenu, Kbd, useMarking, type Mark, KbdChord, Pick, Select, SendKbd, Tabs, Tooltip, boundKeyFor, digitKeyFor, isPressed, pressedDigit, sendsOn, useShortcutReveal, type DispatchKind, type DropdownMenuEntry } from "@armada/components";
import { FIND_A_REPOSITORY, rootOf } from "@armada/shell";

import { KIND_ICON, KIND_LABEL } from "./kinds";
import { useAtMenu, SessionMenu, type Offered, type Tagging } from "./SessionMenu";
import { WaitChip } from "./WaitChip";
import { tagRange, type WaitChip as Chip } from "./waits";
import "./dispatch-panel.css";

/** What the chip says about the repository. A name is a repository; the rest are the ways there is none yet. */
export type PanelReading =
  | { state: "idle" }
  | { state: "detecting" }
  | { state: "unresolved" }
  /** Read from the words, picked by hand, or the one the rail is on. */
  | { state: "detected" | "picked" | "scoped"; name: string };

export type RepositoryOption = {
  root: string;
  name: string;
  /** Ranked ahead of the rest: named by the words, or where the work has been going. */
  likely: boolean;
};

/** One repository on the row. `chosen` is the one the act would use; `read` is the one the words name, where another is chosen. */
export type RepositoryChip = {
  root: string;
  name: string;
  mark?: "chosen" | "read";
};

/** How many chips carry a key. Past them a trailing menu holds the rest. */
const KEYED = 9;

const WHAT: Record<PanelReading["state"], { name: string; tip: string }> = {
  idle: { name: "Repository", tip: "Repository" },
  detecting: { name: "Repository, finding", tip: "Finding the repository" },
  unresolved: { name: "Repository, not found", tip: "No repository found" },
  detected: { name: "Repository", tip: "Found in the request" },
  picked: { name: "Repository", tip: "Picked by you" },
  scoped: { name: "Repository", tip: "The one the rail is on" },
};

/** A Session's Model and Effort as the composer offers them. `null` is Auto. */
export type PanelTuning = {
  models: readonly string[];
  /** The model a Session follows while it names none. */
  defaultModel?: string;
  efforts: readonly string[];
  model: string | null;
  effort: string | null;
  onTune: (tuning: { model: string | null; effort: string | null }) => void;
};

/** What the request can wait on: the live Sessions the `@` offers, and the sentence over the request where the words tag one. */
export type PanelWait = {
  sessions: readonly Offered[];
  chip?: Chip;
};

/** What `@` offers after the Sessions and `#` offers, for the repository the request is for. Absent until one is known. */
export type PanelTags = Pick<Tagging, "searchFiles" | "numbered" | "onTag">;

/** `queue_session`'s binding drawn in Queue for later while ⌘ is held, as Start's is, in whatever key it is bound to now. */
function QueueKbd({ available }: { available: boolean }) {
  const revealing = useShortcutReveal();
  const bound = boundKeyFor("queue_session");
  if (!revealing || !available || bound === "") return null;
  const mods = [...bound].filter((one, at) => "⌃⌥⇧⌘".includes(one) && at < 4);
  const key = bound.slice(mods.length);
  return (
    <span aria-hidden>
      <KbdChord keys={[...mods, key === "Enter" ? "↵" : key]} />
    </span>
  );
}

export type PanelProps = {
  /** The frame grown. Closed it is the line alone. */
  open: boolean;
  words: string;
  onWords: (words: string) => void;
  placeholder: string;
  kind: DispatchKind;
  kinds: readonly DispatchKind[];
  onKind: (kind: DispatchKind) => void;
  reading: PanelReading;
  /** The set-up repositories as the row draws them, in order: the first nine carry ⌥1 to ⌥9. */
  chips: readonly RepositoryChip[];
  /** Every repository, for the trailing menu past nine. */
  entries: readonly DropdownMenuEntry[];
  /** Dispatch is waiting on a repository: the row has the keys. */
  asking: boolean;
  onAsking: (asking: boolean) => void;
  onPick: (root: string) => void;
  /** The act's name: Dispatch, Start, Read it, Start Autopilot. */
  act: string;
  /** The act pressed and held until the repository is read, or until Fleet answers. */
  sending: boolean;
  onAct: () => void;
  /** A Session's Model and Effort, beside the act. Absent for an Autopilot. */
  tuning?: PanelTuning;
  /** The `@` menu and the sentence over the request. Absent where a Session cannot wait. */
  wait?: PanelWait;
  tags?: PanelTags;
  /** The sheet was opened by ⌥A with an `@` already typed: the focus goes to the request and the menu opens once it is drawn, and `onAfterFocused` says it has. */
  focusAfter?: boolean;
  onAfterFocused?: () => void;
  /** Queue for later: saves a Session's idea without starting it. Absent where the queue is not served, and for every other kind. */
  onQueue?: () => void;
  /** Queue for later pressed and held until the repository is read. */
  queueing?: boolean;
  /** Esc. The draft stays. */
  onCollapse: () => void;
  /** A press on the scrim. */
  onDismiss?: () => void;
  /** The host's keys on the request: Tab's kind and a Session's Enter. */
  onFieldKeyDown?: (event: KeyboardEvent<HTMLTextAreaElement>) => void;
  /** Under the band: an Autopilot's read settings. Absent for a Session. */
  body?: ReactNode;
  fieldRef?: Ref<HTMLTextAreaElement>;
  autoFocus?: boolean;
  disabled?: boolean;
};

/** The words, with the tag of the Session the sentence names lit where it stands. The mirror under the request draws it, in the request's own metrics. */
function tagged(words: string, title: string | undefined): ReactNode {
  const range = title === undefined ? undefined : tagRange(words, title);
  if (range === undefined) return words;
  return (
    <>
      {words.slice(0, range[0])}
      <mark className="armada-dispatch-panel__tag">{words.slice(range[0], range[1])}</mark>
      {words.slice(range[1])}
    </>
  );
}

/** `dispatch-panel.css` lifts the menus portalled onto the body above the panel while it is open. */
const DISPATCHING: readonly Mark[] = [{ of: "body", as: "data-dispatching" }];

export function DispatchPanel({
  open,
  words,
  onWords,
  placeholder,
  kind,
  kinds,
  onKind,
  reading,
  chips,
  entries,
  asking,
  onAsking,
  onPick,
  act,
  sending,
  onAct,
  tuning,
  wait,
  tags,
  focusAfter = false,
  onAfterFocused,
  onQueue,
  queueing = false,
  onCollapse,
  onDismiss,
  onFieldKeyDown,
  body,
  fieldRef,
  autoFocus = false,
  disabled = false,
}: PanelProps) {
  const dispatching = useMarking(DISPATCHING);
  const field = useRef<HTMLTextAreaElement | null>(null);
  const row = useRef<HTMLDivElement>(null);
  const frame = useRef<HTMLElement>(null);
  const asked = useRef(false);
  const mirror = useRef<HTMLDivElement>(null);

  // The row takes the keys when the act asks, and gives them back to the request when it stops.
  useEffect(() => {
    if (asking) (row.current?.querySelector<HTMLElement>('[role="radio"][aria-checked="true"]') ?? row.current?.querySelector<HTMLElement>('[role="radio"]'))?.focus();
    else if (asked.current) field.current?.focus();
    asked.current = asking;
  }, [asking]);

  const pick = (root: string) => {
    onPick(root);
    if (!asking) field.current?.focus();
  };

  const sessions = wait?.sessions ?? [];
  const at = useAtMenu(words, onWords, kind === "session" ? { ...tags, sessions } : { sessions: [] }, `${useId()}-at`);

  // ⌥A pressed outside the sheet left an `@` at the end of the request: the caret goes after it and the menu opens.
  useEffect(() => {
    if (!focusAfter || field.current === null) return;
    const el = field.current;
    el.focus();
    el.setSelectionRange(el.value.length, el.value.length);
    at.evaluate(el);
    onAfterFocused?.();
  });

  /** ⌥A in the sheet: an `@` at the caret, a space before it where a word stands there, and the menu open. */
  const typeAt = () => {
    const el = field.current;
    if (el === null) return;
    const caret = el.selectionStart ?? words.length;
    const lead = caret > 0 && !/\s/.test(words[caret - 1] ?? "") ? " " : "";
    onWords(`${words.slice(0, caret)}${lead}@${words.slice(caret)}`);
    const landed = caret + lead.length + 1;
    requestAnimationFrame(() => {
      el.focus();
      el.setSelectionRange(landed, landed);
      at.evaluate(el);
    });
  };

  /** Shift+Tab from the request goes to the sentence above it. */
  const toChip = () => frame.current?.querySelector<HTMLElement>(".armada-wait__part:not(:disabled)")?.focus();

  const keys = (event: KeyboardEvent<HTMLElement>) => {
    // `dispatch_after`, read by the key's place on the keyboard since Option changes the character A types.
    if (isPressed("dispatch_after", event) && wait !== undefined && !disabled) {
      event.preventDefault();
      event.stopPropagation();
      if (kind !== "session") onKind("session");
      return typeAt();
    }
    // `dispatch_repository`'s row of digits, rebindable; Option changes the character a digit types, so it is read by its place on the keyboard.
    const digit = pressedDigit("dispatch_repository", event);
    const target = digit === null ? undefined : chips[digit - 1];
    if (target !== undefined && !disabled) {
      event.preventDefault();
      event.stopPropagation();
      return pick(target.root);
    }
    if (event.key !== "Escape" || event.defaultPrevented) return;
    if (asking) {
      event.preventDefault();
      event.stopPropagation();
      onAsking(false);
    } else if (open) {
      event.preventDefault();
      event.stopPropagation();
      onCollapse();
    }
  };

  const onRow = (event: KeyboardEvent<HTMLDivElement>) => {
    const step = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : 0;
    if (step === 0) return;
    event.preventDefault();
    const all = [...event.currentTarget.querySelectorAll<HTMLElement>('[role="radio"]')];
    const at = all.indexOf(document.activeElement as HTMLElement);
    all[(at + step + all.length) % all.length]?.focus();
  };

  const what = WHAT[reading.state];
  const Mark = KIND_ICON[kind];
  const keyed = chips.slice(0, KEYED);
  const rest = chips.slice(KEYED);
  const hidden = rest.find((one) => one.mark === "chosen");
  const tabbable = (keyed.find((one) => one.mark === "chosen") ?? keyed[0])?.root;

  const repositories = (
    <div
      ref={row}
      className="armada-dispatch-panel__repos"
      role="radiogroup"
      aria-label="Repository"
      onKeyDown={onRow}
    >
      <Tooltip label={what.tip}>
        <span className="armada-dispatch-panel__reading" role="img" aria-label={"name" in reading ? `${what.name} ${reading.name}` : what.name} data-reading={reading.state}>
          {reading.state === "detecting" ? <LoaderCircle size={12} aria-hidden /> : <FolderGit2 size={12} aria-hidden />}
        </span>
      </Tooltip>
      {keyed.map((one, at) => (
        <button
          key={one.root}
          type="button"
          role="radio"
          aria-checked={one.mark === "chosen"}
          aria-label={`Repository ${one.name}`}
          tabIndex={one.root === tabbable ? 0 : -1}
          className="armada-dispatch-panel__repo"
          data-mark={one.mark}
          disabled={disabled}
          onClick={() => pick(one.root)}
        >
          <span className="armada-dispatch-panel__name">{one.name}</span>
          <Kbd aria-hidden>{digitKeyFor("dispatch_repository", at + 1) ?? ""}</Kbd>
        </button>
      ))}
      {rest.length === 0 ? null : (
        <span className="armada-dispatch-panel__more" data-mark={hidden === undefined ? undefined : "chosen"}>
          <DropdownMenu
            align="start"
            portal
            triggerLabel={hidden?.name ?? "More"}
            triggerAriaLabel={hidden === undefined ? "More repositories" : `Repository ${hidden.name}`}
            searchLabel={FIND_A_REPOSITORY}
            entries={[...entries]}
            disabled={disabled}
            onSelect={(id) => pick(rootOf(id))}
          />
        </span>
      )}
    </div>
  );

  const frameOf = (
    <section
      ref={frame}
      className="armada-dispatch-panel__frame"
      aria-label="Dispatch"
      data-open={open || undefined}
      data-kind={kind}
      data-reading={reading.state}
      data-asking={asking || undefined}
      onKeyDown={keys}
    >
      {kind !== "session" || wait?.chip === undefined ? null : <WaitChip chip={wait.chip} sessions={sessions} disabled={disabled} onLeave={() => field.current?.focus()} />}
      <div className="armada-dispatch-panel__line">
        <Tooltip label={KIND_LABEL[kind]}>
          <span className="armada-dispatch-panel__prompt">
            <Mark size={20} aria-hidden />
          </span>
        </Tooltip>
        <div className="armada-dispatch-panel__words">
          <div className="armada-dispatch-panel__mirror" aria-hidden ref={mirror}>
            {tagged(words, kind === "session" ? wait?.chip?.on.title : undefined)}
          </div>
          <textarea
            ref={(element) => {
              field.current = element;
              if (typeof fieldRef === "function") fieldRef(element);
              else if (fieldRef) fieldRef.current = element;
            }}
            aria-label="Request"
            className="armada-dispatch-panel__field"
            rows={1}
            autoFocus={autoFocus}
            value={words}
            placeholder={placeholder}
            aria-controls={at.open ? at.listId : undefined}
            aria-activedescendant={at.activeId}
            onChange={(event) => {
              onWords(event.target.value);
              at.onChange(event);
            }}
            onSelect={at.onSelect}
            onBlur={at.onBlur}
            onScroll={(event) => mirror.current !== null && (mirror.current.scrollTop = event.currentTarget.scrollTop)}
            onKeyDown={(event) => {
              if (at.onKeyDown(event)) return;
              if (event.key === "Tab" && event.shiftKey && kind === "session" && wait?.chip !== undefined) {
                event.preventDefault();
                return toChip();
              }
              onFieldKeyDown?.(event);
              if (event.defaultPrevented || !open || kind === "session" || !sendsOn(event)) return;
              event.preventDefault();
              onAct();
            }}
          />
        </div>
        {kinds.length < 2 ? null : (
          <Tooltip label={kinds.map((one) => KIND_LABEL[one]).join(" / ")}>
            <Kbd className="armada-dispatch-panel__tab">⇥</Kbd>
          </Tooltip>
        )}
        <Tooltip label="Dispatch" shortcut="N">
          <Kbd>N</Kbd>
        </Tooltip>
      </div>

      {!at.open ? null : <SessionMenu id={at.listId} label={at.label} numbered={at.numbered} rows={at.offered} active={at.active} onHover={at.setActive} onChoose={at.choose} />}

      <div className="armada-dispatch-panel__drawer" inert={!open}>
        <div className="armada-dispatch-panel__drawer-body">
          {repositories}

          <div className="armada-dispatch-panel__band">
            {kinds.length < 2 ? null : (
              <Tabs items={kinds.map((one) => ({ id: one, label: KIND_LABEL[one] }))} value={kind} onChange={(id) => onKind(id as DispatchKind)} />
            )}
            <span className="armada-dispatch-panel__send">
              {tuning === undefined ? null : (
                <span className="armada-dispatch-panel__picks">
                  <Pick Glyph={Cpu} compact label="Model">
                    <Select aria-label="Model" value={tuning.model ?? ""} disabled={disabled} onChange={(event) => tuning.onTune({ model: event.target.value === "" ? null : event.target.value, effort: tuning.effort })}>
                      <option value="">{tuning.defaultModel ?? "Auto"}</option>
                      {tuning.models.map((one) => (
                        <option key={one} value={one}>
                          {one}
                        </option>
                      ))}
                    </Select>
                  </Pick>
                  <Pick Glyph={Zap} compact label="Effort">
                    <Select aria-label="Effort" value={tuning.effort ?? ""} disabled={disabled} onChange={(event) => tuning.onTune({ model: tuning.model, effort: event.target.value === "" ? null : event.target.value })}>
                      <option value="">Auto</option>
                      {tuning.efforts.map((one) => (
                        <option key={one} value={one}>
                          {one}
                        </option>
                      ))}
                    </Select>
                  </Pick>
                </span>
              )}
              {onQueue === undefined ? null : (
                <>
                  <QueueKbd available={words.trim() !== ""} />
                  <Button variant="secondary" size="sm" pending={queueing} disabled={disabled || sending || words.trim() === ""} onClick={onQueue}>
                    Queue for later
                  </Button>
                </>
              )}
              <SendKbd available={words.trim() !== ""} />
              <Button variant="primary" size="sm" pending={sending} disabled={disabled || queueing || words.trim() === ""} onClick={onAct}>
                {act}
              </Button>
            </span>
          </div>

          {body === undefined ? null : <div className="armada-dispatch-panel__settings">{body}</div>}
        </div>
      </div>
    </section>
  );

  return (
    <div
      ref={dispatching}
      className="armada-dispatch-panel"
      data-settles
      role="dialog"
      aria-modal="true"
      aria-label="Dispatch"
      onPointerDown={(event) => event.target === event.currentTarget && onDismiss?.()}
    >
      {frameOf}
    </div>
  );
}

/** Where a frame hangs from a field: the box of the field it grew from, read once as it opens. The strip a send leaves hangs from the title bar's field. */
export function useHung(on: boolean, anchor: string | undefined): CSSProperties | undefined {
  const [place, setPlace] = useState<CSSProperties>();
  useLayoutEffect(() => {
    const box = !on || anchor === undefined ? undefined : document.querySelector(anchor)?.getBoundingClientRect();
    if (box === undefined || box.width === 0) return setPlace(undefined);
    setPlace({ ["--hung-top" as string]: `${box.top}px`, ["--hung-left" as string]: `${box.left}px`, ["--hung-width" as string]: `${box.width}px`, ["--hung-height" as string]: `${box.height}px` });
  }, [on, anchor]);
  return place;
}
