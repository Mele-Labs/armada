// The dispatch panel: the place work is described, grown down from the title bar's field when the one
// bar chose to dispatch. One frame holds it: the line (prompt, request, the key that switches kind)
// never moves, and a drawer under it opens for the band (kind, repository, the act) and a body that depends on the kind: a Job's
// Settings, an Autopilot's read settings, nothing for a Session. The request is one textarea that is
// never remounted, so what was typed and the caret stay where they were while the frame grows.
//
// The repository is an option in the band. A Job and an Autopilot need one, and when none was read
// from the words the act asks inline, in the frame, before anything is built; a Session never asks.
// State is carried by the frame (its edge and hue) and by the chip's icon, not by a phrase.

import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type KeyboardEvent, type ReactNode, type Ref } from "react";
import { FolderGit2, LoaderCircle } from "lucide-react";
import { Button, Kbd, SendKbd, Tabs, Tooltip, sendsOn, type DispatchKind } from "@armada/components";

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

const WHAT: Record<PanelReading["state"], { name: string; tip: string }> = {
  idle: { name: "Repository", tip: "Repository" },
  detecting: { name: "Repository, finding", tip: "Finding the repository" },
  unresolved: { name: "Repository, not found", tip: "No repository found" },
  detected: { name: "Repository", tip: "Found in the request. Press to change" },
  picked: { name: "Repository", tip: "Picked by you. Press to change" },
  scoped: { name: "Repository", tip: "The one the rail is on. Press to change" },
};

export type PanelProps = {
  /** The box the frame hangs from: a selector, read when the panel opens. */
  anchor?: string;
  /** The frame grown. Closed it is the line alone. */
  open: boolean;
  words: string;
  onWords: (words: string) => void;
  placeholder: string;
  kind: DispatchKind;
  kinds: readonly DispatchKind[];
  onKind: (kind: DispatchKind) => void;
  reading: PanelReading;
  /** The set-up repositories, likely ones first. */
  options: readonly RepositoryOption[];
  /** The inline list of repositories is open. */
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
  anchor,
  open,
  words,
  onWords,
  placeholder,
  kind,
  kinds,
  onKind,
  reading,
  options,
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
  const list = useRef<HTMLUListElement>(null);
  const asked = useRef(false);
  const place = useHung(true, anchor);

  // The list takes the keys when it opens and gives them back to the request when it closes.
  useEffect(() => {
    if (asking) list.current?.querySelector<HTMLElement>('[role="option"]')?.focus();
    else if (asked.current) field.current?.focus();
    asked.current = asking;
  }, [asking]);

  const keys = (event: KeyboardEvent<HTMLElement>) => {
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

  const onList = (event: KeyboardEvent<HTMLUListElement>) => {
    const step = event.key === "ArrowDown" ? 1 : event.key === "ArrowUp" ? -1 : 0;
    if (step === 0) return;
    event.preventDefault();
    const all = [...event.currentTarget.querySelectorAll<HTMLElement>('[role="option"]')];
    const at = all.indexOf(document.activeElement as HTMLElement);
    all[(at + step + all.length) % all.length]?.focus();
  };

  const what = WHAT[reading.state];
  const named = "name" in reading;
  const Mark = KIND_ICON[kind];
  const chip = (
    <button
      type="button"
      className="armada-dispatch-panel__chip"
      data-reading={reading.state}
      aria-label={named ? `${what.name} ${reading.name}` : what.name}
      aria-haspopup="listbox"
      aria-expanded={asking}
      disabled={disabled}
      onClick={() => onAsking(!asking)}
    >
      {reading.state === "detecting" ? <LoaderCircle size={12} aria-hidden /> : <FolderGit2 size={12} aria-hidden />}
      {named ? <span className="armada-dispatch-panel__name">{reading.name}</span> : null}
    </button>
  );

  const frameOf = (
    <section
      className="armada-dispatch-panel__frame"
      aria-label="Dispatch"
      data-open={open || undefined}
      data-kind={kind}
      data-reading={reading.state}
      data-asking={asking || undefined}
      style={place}
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
          <div className="armada-dispatch-panel__band">
            {kinds.length < 2 ? null : (
              <Tabs items={kinds.map((one) => ({ id: one, label: KIND_LABEL[one] }))} value={kind} onChange={(id) => onKind(id as DispatchKind)} />
            )}
            <Tooltip label={what.tip}>{chip}</Tooltip>
            <span className="armada-dispatch-panel__send">
              <SendKbd available={words.trim() !== ""} />
              <Button variant="primary" size="sm" pending={sending} disabled={disabled || words.trim() === ""} onClick={onAct}>
                {act}
              </Button>
            </span>
          </div>

          <div className="armada-dispatch-panel__ask" data-open={asking || undefined} inert={!asking}>
            <ul ref={list} className="armada-dispatch-panel__options" role="listbox" aria-label="Repository" onKeyDown={onList}>
              {options.map((one, at) => (
                <li key={one.root} role="none" data-edge={one.likely && !options[at + 1]?.likely ? "" : undefined}>
                  <button
                    type="button"
                    role="option"
                    aria-selected={reading.state !== "unresolved" && "name" in reading && reading.name === one.name}
                    className="armada-dispatch-panel__option"
                    data-likely={one.likely || undefined}
                    onClick={() => onPick(one.root)}
                  >
                    <FolderGit2 size={12} aria-hidden />
                    <span className="armada-dispatch-panel__name">{one.name}</span>
                    <span className="armada-dispatch-panel__root">{one.root}</span>
                  </button>
                </li>
              ))}
            </ul>
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

/** Where an anchored frame hangs: the box of the field it grew from, read once as the panel opens. */
export function useHung(on: boolean, anchor: string | undefined): CSSProperties | undefined {
  const [place, setPlace] = useState<CSSProperties>();
  useLayoutEffect(() => {
    const box = !on || anchor === undefined ? undefined : document.querySelector(anchor)?.getBoundingClientRect();
    if (box === undefined || box.width === 0) return setPlace(undefined);
    setPlace({ ["--hung-top" as string]: `${box.top}px`, ["--hung-left" as string]: `${box.left}px`, ["--hung-width" as string]: `${box.width}px`, ["--hung-height" as string]: `${box.height}px` });
  }, [on, anchor]);
  return place;
}
