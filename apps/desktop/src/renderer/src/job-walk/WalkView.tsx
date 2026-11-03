// A walk as its Job's tab draws it: the bar at the top, then the page under it. It takes plain props and assumes nothing of the tab around it, so the review sheet can host it too.
// **The page is a slot.** The mock has no native page view, so it draws a fixture there and rings the step's target in DOM; the real wiring lays Bridge's own page over the same area.

import { useLayoutEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { Button, Textarea, WalkBar } from "@armada/components";

import "./job-walk.css";

export type WalkViewProps = {
  /** The walk's name, which is its page's. */
  name: string;
  /** The page's takes by name, and the one on screen. Absent: one page. */
  takes?: readonly string[];
  take: number;
  onTake: (at: number) => void;
  steps: readonly {
    say: string;
    target: string;
  }[];
  /** The step on screen, from 0. */
  step: number;
  onStep: (to: number) => void;
  /** Sends a note to the Job the walk belongs to. */
  onNote: (text: string) => void;
  onApprove: () => void;
  /** Names what the note's text is kept under while it is typed. */
  noteKey: string;
  /** The page, drawn in the area under the bar. Mock only. */
  page: ReactNode;
};

type Box = {
  x: number;
  y: number;
  w: number;
  h: number;
};

export function WalkView({
  name,
  takes,
  take,
  onTake,
  steps,
  step,
  onStep,
  onNote,
  onApprove,
  noteKey,
  page,
}: WalkViewProps) {
  const area = useRef<HTMLDivElement>(null);
  const [box, setBox] = useState<Box | null>(null);
  const [loads, setLoads] = useState(0);
  const [noting, setNoting] = useState(false);
  const [text, setText] = useState("");
  const aimed = steps[step]?.target;

  // The ring is measured after every draw, and kept only when it moved.
  useLayoutEffect(() => {
    const host = area.current;
    const target =
      aimed === undefined
        ? null
        : host?.querySelector<HTMLElement>(`[data-walk-target="${aimed}"]`);
    if (host === null || target === null || target === undefined) return setBox(null);
    const from = host.getBoundingClientRect();
    const at = target.getBoundingClientRect();
    const next = {
      x: Math.round(at.left - from.left + host.scrollLeft),
      y: Math.round(at.top - from.top + host.scrollTop),
      w: Math.round(at.width),
      h: Math.round(at.height),
    };
    setBox((was) =>
      was !== null && was.x === next.x && was.y === next.y && was.w === next.w && was.h === next.h
        ? was
        : next,
    );
  });

  const close = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key !== "Escape") return;
    event.preventDefault();
    event.stopPropagation();
    setNoting(false);
  };

  return (
    <div className="job-walk">
      <WalkBar
        name={name}
        {...(takes === undefined || takes.length === 0
          ? {
              onReload: () => setLoads((one) => one + 1),
            }
          : {
              takes: {
                names: takes,
                at: take,
                onPick: onTake,
              },
            })}
        at={step + 1}
        of={steps.length}
        said={steps[step]?.say ?? ""}
        onBack={() => onStep(Math.max(0, step - 1))}
        onNext={() => onStep(Math.min(steps.length - 1, step + 1))}
        noting={noting}
        onNote={() => setNoting((one) => !one)}
        onApprove={onApprove}
      />
      <div className="job-walk__page" ref={area}>
        <div key={loads}>{page}</div>
        {box === null ? null : (
          <div
            className="job-walk__ring"
            role="img"
            aria-label="The target of this step"
            style={{
              left: box.x,
              top: box.y,
              width: box.w,
              height: box.h,
            }}
          />
        )}
        {!noting ? null : (
          <form
            className="job-walk__note"
            aria-label="Note"
            onKeyDown={close}
            onSubmit={(event) => {
              event.preventDefault();
              if (text.trim() === "") return;
              onNote(text.trim());
              setText("");
              setNoting(false);
            }}
          >
            <Textarea
              label="Note"
              rows={3}
              keep={`walk-note:${noteKey}`}
              value={text}
              onChange={(event) => setText(event.target.value)}
              // eslint-disable-next-line jsx-a11y/no-autofocus
              autoFocus
            />
            <div className="job-walk__note-acts">
              <Button variant="ghost" size="sm" type="button" onClick={() => setNoting(false)}>
                Close
              </Button>
              <Button variant="primary" size="sm" type="submit" disabled={text.trim() === ""}>
                Send
              </Button>
            </div>
          </form>
        )}
      </div>
    </div>
  );
}
