// The Judge act: a balance on an artifact's row, in the Thread's head for the whole Job, and on the plan card. A press opens a small popover that asks what
// to verify; nothing is read until Judge is pressed in it. The icon then reads the state of the last reading: a pulsing mark while it reads, a dot while its
// notes are unread, a check after a clean one. No count is drawn.

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { CircleCheck, CircleDot, Scale } from "lucide-react";
import { Button, Textarea, Tooltip } from "@armada/components";

import { keyOf, registerOpener, useJudgeApi, useJudgeRecord, type JudgeArtifact } from "./store";
import "./judge.css";

const PLACEHOLDER = "existing sign-ins survive the switch";

type Place = { top: number; left: number };

/** The popover, drawn on the body so a glass pane cannot clip it, under the act and lined up with its right edge. */
function JudgePopover({ job, artifact, anchor, onRun, onClose }: { job: string; artifact: JudgeArtifact; anchor: HTMLElement; onRun: (prompt: string) => void; onClose: () => void }) {
  const [text, setText] = useState("");
  const [place, setPlace] = useState<Place | undefined>();
  const panel = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const at = anchor.getBoundingClientRect();
    const width = panel.current?.offsetWidth ?? 0;
    setPlace({ top: at.bottom + 4, left: Math.max(8, Math.min(at.right - width, window.innerWidth - width - 8)) });
  }, [anchor]);
  useEffect(() => {
    const field = panel.current?.querySelector("textarea");
    field?.focus();
    field?.setSelectionRange(field.value.length, field.value.length);
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      onClose();
    };
    const onDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (!panel.current?.contains(target) && !anchor.contains(target)) onClose();
    };
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("mousedown", onDown);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("mousedown", onDown);
    };
  }, [anchor, onClose]);
  const run = () => {
    onRun(text.trim());
    setText("");
  };
  return createPortal(
    // A portal's events bubble through the React tree to the card or row that holds the act, which must not hear a key or a press made in the popover.
    <div ref={panel} className="judge-pop" role="dialog" aria-label="Judge" style={place === undefined ? { visibility: "hidden" } : place} onClick={(event) => event.stopPropagation()} onKeyDown={(event) => event.stopPropagation()}>
      <Textarea
        label="Verify that"
        keep={`judge:${job}:${keyOf(artifact)}`}
        placeholder={PLACEHOLDER}
        rows={3}
        value={text}
        onChange={(event) => setText(event.target.value)}
        onKeyDown={(event) => {
          if (event.key !== "Enter" || event.shiftKey || event.nativeEvent.isComposing) return;
          event.preventDefault();
          run();
        }}
      />
      <p className="judge-pop__hint">Empty: what did we miss?</p>
      <div className="judge-pop__acts">
        <Button size="sm" variant="primary" onClick={run}>
          Judge
        </Button>
      </div>
    </div>,
    document.body,
  );
}

/**
 * `label` names the press for a reader of the screen (`Judge #2390`); `size` is the glyph's. `shortcut` is the key that opens the popover, shown in the tooltip.
 * A window with no judge to ask draws nothing.
 */
export function JudgeAct({ job, artifact, label = "Judge", size = 16, shortcut }: { job: string; artifact: JudgeArtifact; label?: string; size?: number; shortcut?: string }) {
  const api = useJudgeApi();
  const record = useJudgeRecord(job, artifact);
  const [open, setOpen] = useState(false);
  const button = useRef<HTMLButtonElement>(null);
  useEffect(() => registerOpener(job, artifact, () => setOpen(true)), [job, artifact.kind, artifact.id]);
  if (api === undefined) return null;
  const reading = record?.state === "reading";
  const unread = record?.state === "done" && !record.read && record.notes.some((one) => one.settled === undefined);
  const clean = record?.state === "done" && record.notes.length === 0;
  const close = () => {
    setOpen(false);
    button.current?.focus();
  };
  const said = reading ? "Judge reading" : unread ? "Notes to read" : clean ? "Nothing missed" : "Send a judge to read it";
  return (
    <>
      <Tooltip label={said} {...(shortcut === undefined ? {} : { shortcut })} asChild>
        <Button ref={button} variant="ghost" size="sm" iconOnly aria-label={label} aria-haspopup="dialog" aria-expanded={open} aria-busy={reading || undefined} data-judge={artifact.kind} onClick={() => setOpen(!open)}>
          {reading ? (
            <span className="armada-session-mark" role="img" aria-label="Judge reading" data-pulsing>
              <CircleDot size={size} strokeWidth={2} aria-hidden="true" />
            </span>
          ) : clean ? (
            <span className="armada-session-mark judge-act__clean" role="img" aria-label="Nothing missed">
              <CircleCheck size={size} strokeWidth={2} aria-hidden="true" />
            </span>
          ) : (
            <Scale size={size} strokeWidth={2} aria-hidden="true" />
          )}
          {unread ? <span className="judge-act__dot" role="img" aria-label="Notes to read" /> : null}
        </Button>
      </Tooltip>
      {open && button.current !== null ? (
        <JudgePopover
          job={job}
          artifact={artifact}
          anchor={button.current}
          onClose={close}
          onRun={(prompt) => {
            void api.ask({ job, artifact, prompt });
            close();
          }}
        />
      ) : null}
    </>
  );
}
