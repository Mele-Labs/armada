// The dispatch panel: the place work is described, a centred sheet over the window that `n`, the Cockpit's
// Dispatch tile, every New job and a dispatch row chosen in the palette all open. One frame holds it: the line (prompt, request, the key that switches kind)
// never moves, and a drawer under it opens for the band (kind, repository, the act) and a body that depends on the kind: a Job's
// Settings, an Autopilot's read settings, nothing for a Session. The request is one textarea that is
// never remounted, so what was typed and the caret stay where they were while the frame grows.
//
// The repository is a row of chips under the request, never a dropdown: each carries its key, ⌥1 to ⌥9, and
// the one the words name lights as they are typed. Past nine a trailing menu holds the rest. When none is
// chosen the act gives the row the keys (← and →, ↵ picks and sends) before anything is built.
// State is carried by the frame (its edge and hue) and by the chosen chip's fill, not by a phrase.

import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type KeyboardEvent, type ReactNode, type Ref } from "react";
import { FolderGit2, LoaderCircle } from "lucide-react";
import { Button, DropdownMenu, Kbd, SendKbd, Tabs, Tooltip, digitKeyFor, pressedDigit, sendsOn, type DispatchKind, type DropdownMenuEntry } from "@armada/components";
import { FIND_A_REPOSITORY, rootOf } from "@armada/shell";

import { KIND_ICON, KIND_LABEL } from "./kinds";
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
  /** Esc. The draft stays. */
  onCollapse: () => void;
  /** A press on the scrim. */
  onDismiss?: () => void;
  /** The host's keys on the request: Tab's kind and a Session's Enter. */
  onFieldKeyDown?: (event: KeyboardEvent<HTMLTextAreaElement>) => void;
  /** Under the band: a Job's Settings, an Autopilot's read settings. Absent for a Session. */
  body?: ReactNode;
  fieldRef?: Ref<HTMLTextAreaElement>;
  autoFocus?: boolean;
  disabled?: boolean;
};

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
  onCollapse,
  onDismiss,
  onFieldKeyDown,
  body,
  fieldRef,
  autoFocus = false,
  disabled = false,
}: PanelProps) {
  const field = useRef<HTMLTextAreaElement | null>(null);
  const row = useRef<HTMLDivElement>(null);
  const asked = useRef(false);

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

  const keys = (event: KeyboardEvent<HTMLElement>) => {
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
      className="armada-dispatch-panel__frame"
      aria-label="Dispatch"
      data-open={open || undefined}
      data-kind={kind}
      data-reading={reading.state}
      data-asking={asking || undefined}
      onKeyDown={keys}
    >
      <div className="armada-dispatch-panel__line">
        <Tooltip label={KIND_LABEL[kind]}>
          <span className="armada-dispatch-panel__prompt">
            <Mark size={20} aria-hidden />
          </span>
        </Tooltip>
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
          onChange={(event) => onWords(event.target.value)}
          onKeyDown={(event) => {
            onFieldKeyDown?.(event);
            if (event.defaultPrevented || !open || kind === "session" || !sendsOn(event)) return;
            event.preventDefault();
            onAct();
          }}
        />
        {kinds.length < 2 ? null : (
          <Tooltip label={kinds.map((one) => KIND_LABEL[one]).join(" / ")}>
            <Kbd className="armada-dispatch-panel__tab">⇥</Kbd>
          </Tooltip>
        )}
        <Tooltip label="Dispatch" shortcut="N">
          <Kbd>N</Kbd>
        </Tooltip>
      </div>

      <div className="armada-dispatch-panel__drawer" inert={!open}>
        <div className="armada-dispatch-panel__drawer-body">
          {repositories}

          <div className="armada-dispatch-panel__band">
            {kinds.length < 2 ? null : (
              <Tabs items={kinds.map((one) => ({ id: one, label: KIND_LABEL[one] }))} value={kind} onChange={(id) => onKind(id as DispatchKind)} />
            )}
            <span className="armada-dispatch-panel__send">
              <SendKbd available={words.trim() !== ""} />
              <Button variant="primary" size="sm" pending={sending} disabled={disabled || words.trim() === ""} onClick={onAct}>
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
