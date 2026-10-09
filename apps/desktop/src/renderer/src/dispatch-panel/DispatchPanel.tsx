// The dispatch panel: the Dashboard's dispatch bar grown into the place a Job is described, and the
// same panel floating over any other screen. One frame holds both: the line (prompt, request, the
// Job / Session choice) never moves, and a drawer under it opens for the repository, Settings and
// Dispatch. The request is one textarea that is never remounted, so what was typed and the caret
// stay where they were while the frame grows.
//
// The repository is an option in the band, never a gate. It reads itself from the words, shows the
// reading in its icon, and when none was found Dispatch asks inline, in the frame, before anything
// is built. State is carried by the frame (its edge and hue) and by the chip's icon, not by a phrase.

import { useEffect, useRef, type KeyboardEvent, type ReactNode, type Ref } from "react";
import { FolderGit2, LoaderCircle, Send } from "lucide-react";
import {
  Button,
  DispatchSettings,
  Kbd,
  SendKbd,
  Tooltip,
  sendsOn,
  type DispatchSettingsProps,
} from "@armada/components";

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

export function DispatchPanel({
  layout,
  open,
  words,
  onWords,
  placeholder,
  reading,
  options,
  asking,
  onAsking,
  onPick,
  settings,
  onSettings,
  workflows,
  leftOut,
  models,
  machineCap,
  settingsOpen,
  onSettingsOpen,
  sending,
  onSend,
  onOpen,
  onCollapse,
  onDismiss,
  onFieldKeyDown,
  lead,
  fieldRef,
  autoFocus = false,
  disabled = false,
}: {
  /** In the Dashboard's slot, or floating over whatever screen is showing. */
  layout: "docked" | "floating";
  /** The frame grown. Closed it is the bar. */
  open: boolean;
  words: string;
  onWords: (words: string) => void;
  placeholder: string;
  reading: PanelReading;
  /** The set-up repositories, likely ones first. */
  options: readonly RepositoryOption[];
  /** The inline list of repositories is open. */
  asking: boolean;
  onAsking: (asking: boolean) => void;
  onPick: (root: string) => void;
  /** Dispatch pressed and held until the repository is read. */
  sending: boolean;
  onSend: () => void;
  /** The frame was pressed while closed. */
  onOpen: () => void;
  /** Esc, or a press outside a docked frame. The draft stays. */
  onCollapse: () => void;
  /** A press on the scrim of the floating layer. */
  onDismiss?: () => void;
  /** The host's keys on the request: a Session's Tab and Enter. */
  onFieldKeyDown?: (event: KeyboardEvent<HTMLTextAreaElement>) => void;
  /** The Job / Session choice, at the line's trailing edge. */
  lead?: ReactNode;
  fieldRef?: Ref<HTMLTextAreaElement>;
  autoFocus?: boolean;
  disabled?: boolean;
} & Pick<DispatchSettingsProps, "settings" | "onSettings" | "workflows" | "models" | "machineCap"> & {
    leftOut?: DispatchSettingsProps["leftOut"];
    settingsOpen: boolean;
    onSettingsOpen: (open: boolean) => void;
  }) {
  const frame = useRef<HTMLElement>(null);
  const field = useRef<HTMLTextAreaElement | null>(null);
  const list = useRef<HTMLUListElement>(null);
  const asked = useRef(false);

  // The list takes the keys when it opens and gives them back to the request when it closes.
  useEffect(() => {
    if (asking) list.current?.querySelector<HTMLElement>('[role="option"]')?.focus();
    else if (asked.current) field.current?.focus();
    asked.current = asking;
  }, [asking]);

  // A docked frame is over the Cockpit, whose tiles are under it: a press out there puts it back.
  useEffect(() => {
    if (layout !== "docked" || !open) return;
    const away = (event: MouseEvent) => {
      if (event.target instanceof Node && frame.current?.contains(event.target)) return;
      onCollapse();
    };
    document.addEventListener("mousedown", away);
    return () => document.removeEventListener("mousedown", away);
  }, [layout, open, onCollapse]);

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
      ref={frame}
      className="armada-dispatch-panel__frame"
      aria-label="Dispatch"
      data-open={open || undefined}
      data-reading={reading.state}
      data-asking={asking || undefined}
      onKeyDown={keys}
      onClick={() => open || onOpen()}
    >
      <div className="armada-dispatch-panel__line">
        <span className="armada-dispatch-panel__prompt" aria-hidden="true">
          <Send size={20} />
        </span>
        <textarea
          ref={(element) => {
            field.current = element;
            if (typeof fieldRef === "function") fieldRef(element);
            else if (fieldRef) fieldRef.current = element;
          }}
          aria-label="Request"
          className="armada-dispatch-panel__field"
          data-layout={layout}
          rows={1}
          autoFocus={autoFocus}
          value={words}
          placeholder={placeholder}
          onChange={(event) => onWords(event.target.value)}
          onKeyDown={(event) => {
            onFieldKeyDown?.(event);
            if (event.defaultPrevented || !open || !sendsOn(event)) return;
            event.preventDefault();
            onSend();
          }}
        />
        {lead}
        <Tooltip label="Dispatch" shortcut="N">
          <Kbd>N</Kbd>
        </Tooltip>
      </div>

      <div className="armada-dispatch-panel__drawer" inert={!open}>
        <div className="armada-dispatch-panel__drawer-body">
          <div className="armada-dispatch-panel__band">
            <Tooltip label={what.tip}>{chip}</Tooltip>
            <span className="armada-dispatch-panel__send">
              <SendKbd available={words.trim() !== ""} />
              <Button
                variant="primary"
                size="sm"
                pending={sending}
                disabled={disabled || words.trim() === ""}
                onClick={onSend}
              >
                Dispatch
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

          <div className="armada-dispatch-panel__settings">
            <DispatchSettings
              open={settingsOpen}
              onOpenChange={onSettingsOpen}
              settings={settings}
              onSettings={onSettings}
              workflows={workflows}
              {...(leftOut === undefined ? {} : { leftOut })}
              models={models}
              machineCap={machineCap}
              disabled={disabled}
            />
          </div>
        </div>
      </div>
    </section>
  );

  if (layout === "docked") {
    return <div className="armada-dispatch-panel" data-layout="docked" data-settles>{frameOf}</div>;
  }
  return (
    <div
      className="armada-dispatch-panel"
      data-layout="floating"
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
