// A walk as its Job's tab draws it: the bar at the top, then the page under it. It takes plain props and assumes nothing of the tab around it, so the review sheet can host it too.
// **The page is a slot.** The mock has no native page view, so it draws a fixture there and rings the step's target in DOM; the real wiring lays Bridge's own page over the same area.
// **A note is taken as the capture window takes one**: Note arms the page, a press picks what is under it, and a card beside the pick holds the words and Capture. The note is held
// on the Job, not sent (`docs/practices/capture-window.md`, *Pending notes*).

import { useLayoutEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import {
  ACTION,
  StudioCapture,
  WalkBar,
  type CaptureBarReadiness,
  type CaptureBox,
  type WalkBarHeld,
} from "@armada/components";
import { described } from "@armada/screens/src/fixtures/walk-window";

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
    target?: string | undefined;
  }[];
  /** The step on screen, from 0. */
  step: number;
  onStep: (to: number) => void;
  /** A note captured: its words, and where it was taken as the walk, the take and the element name it. */
  onNote: (said: string, on: string) => void;
  onApprove: () => void;
  /** Names what the note's text is kept under while it is typed. */
  noteKey: string;
  /** What a note goes to, as the capture bar says it: `Onto <the Job>`. */
  aim: string;
  /** What the Job changed and what to look at. Absent: it said nothing. */
  brief?: string;
  /** Notes held on the Job, which turn Approve into Send notes. */
  held?: WalkBarHeld;
  readiness?: CaptureBarReadiness;
  /** The page, drawn in the area under the bar. Mock only. */
  page: ReactNode;
};

/** The box of `target` in the page area's own pixels, as it is now; with `scrolled`, in the area's scrolled content. */
const boxOf = (host: HTMLElement, target: HTMLElement, scrolled: boolean): CaptureBox => {
  const from = host.getBoundingClientRect();
  const at = target.getBoundingClientRect();
  return {
    x: Math.round(at.left - from.left + (scrolled ? host.scrollLeft : 0)),
    y: Math.round(at.top - from.top + (scrolled ? host.scrollTop : 0)),
    width: Math.round(at.width),
    height: Math.round(at.height),
  };
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
  aim,
  brief,
  held,
  readiness,
  page,
}: WalkViewProps) {
  const area = useRef<HTMLDivElement>(null);
  const content = useRef<HTMLDivElement>(null);
  const [ring, setRing] = useState<CaptureBox | null>(null);
  const [loads, setLoads] = useState(0);
  // A note being taken: the page armed for a press, then what the press picked and the words typed.
  const [armed, setArmed] = useState(false);
  const [hovered, setHovered] = useState<{
    box: CaptureBox;
    named: string;
  } | null>(null);
  const [picked, setPicked] = useState<{
    box: CaptureBox;
    element: string;
  } | null>(null);
  const [said, setSaid] = useState("");
  const aimed = steps[step]?.target;
  const taken = takes?.[take];

  // The step's ring is measured after every draw, and kept only when it moved.
  useLayoutEffect(() => {
    const host = area.current;
    const target =
      aimed === undefined
        ? null
        : host?.querySelector<HTMLElement>(`[data-walk-target="${aimed}"]`);
    if (host === null || target === null || target === undefined) return setRing(null);
    const next = boxOf(host, target, true);
    setRing((was) =>
      was !== null &&
      was.x === next.x &&
      was.y === next.y &&
      was.width === next.width &&
      was.height === next.height
        ? was
        : next,
    );
  });

  /** What the page holds under a point on the cover. */
  const under = (x: number, y: number): HTMLElement | null =>
    (document
      .elementsFromPoint(x, y)
      .find((one) => one !== content.current && content.current?.contains(one)) as
      | HTMLElement
      | undefined) ?? null;

  const pick = (target: HTMLElement) => {
    const host = area.current;
    if (host === null) return;
    setHovered(null);
    setArmed(false);
    setPicked({
      box: boxOf(host, target, false),
      element: described(target),
    });
  };

  const cancel = () => {
    setArmed(false);
    setHovered(null);
    setPicked(null);
    setSaid("");
  };

  const save = () => {
    if (picked === null || said.trim() === "") return;
    onNote(said.trim(), [name, taken, picked.element].filter((one) => one !== undefined).join(", "));
    cancel();
  };

  const capturing = armed || picked !== null;
  const toggle = () => (capturing ? cancel() : setArmed(true));
  const binding = [
    ...(ACTION.capture_note?.shortcut ?? ""),
  ];

  return (
    <div
      className="job-walk"
      onKeyDown={(event: KeyboardEvent<HTMLDivElement>) => {
        // The registry's own key, as the Capture button's tooltip shows it; ⌥ changes what `key` reads, so the key is named by its place.
        if (event.metaKey && event.altKey && !event.ctrlKey && event.code === "KeyC") {
          event.preventDefault();
          event.stopPropagation();
          toggle();
        }
      }}
    >
      <WalkBar
        name={name}
        {...(takes === undefined || takes.length === 0
          ? {}
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
        onReload={() => setLoads((one) => one + 1)}
        capturing={capturing}
        binding={binding}
        onCapture={toggle}
        onApprove={onApprove}
        {...(brief === undefined
          ? {}
          : {
              brief,
            })}
        {...(held === undefined
          ? {}
          : {
              held,
            })}
        {...(readiness === undefined
          ? {}
          : {
              readiness,
            })}
      />
      <div
        className="job-walk__stage"
        onKeyDown={(event: KeyboardEvent<HTMLDivElement>) => {
          if (event.key !== "Escape" || !capturing) return;
          event.preventDefault();
          event.stopPropagation();
          cancel();
        }}
      >
        <div className="job-walk__page" ref={area}>
          <div key={loads} ref={content}>
            {page}
          </div>
          {ring === null || capturing ? null : (
            <div
              className="job-walk__ring"
              role="img"
              aria-label="The target of this step"
              style={{
                left: ring.x,
                top: ring.y,
                width: ring.width,
                height: ring.height,
              }}
            />
          )}
        </div>
        {!armed ? null : (
          <button
            type="button"
            className="job-walk__pick"
            aria-label="Pick on the page"
            // eslint-disable-next-line jsx-a11y/no-autofocus
            autoFocus
            onMouseMove={(event) => {
              const host = area.current;
              const hit = under(event.clientX, event.clientY);
              setHovered(
                host === null || hit === null
                  ? null
                  : {
                      box: boxOf(host, hit, false),
                      named: described(hit),
                    },
              );
            }}
            onClick={(event) => {
              // From the keyboard, which has no point, the press picks the step's ringed target.
              const found =
                event.detail === 0
                  ? aimed === undefined
                    ? null
                    : (area.current?.querySelector<HTMLElement>(`[data-walk-target="${aimed}"]`) ??
                      null)
                  : under(event.clientX, event.clientY);
              if (found !== null) pick(found);
            }}
          />
        )}
        {!capturing ? null : (
          <StudioCapture
            aim={aim}
            aimed
            binding={binding}
            note={said}
            onNote={setSaid}
            onSave={save}
            onCancel={cancel}
            keep={`walk-note:${noteKey}`}
            {...(hovered === null
              ? {}
              : {
                  hovered,
                })}
            {...(picked === null
              ? {}
              : {
                  held: {
                    box: picked.box,
                    chain: [name, taken, picked.element]
                      .filter((one) => one !== undefined)
                      .join(", "),
                  },
                })}
          />
        )}
      </div>
    </div>
  );
}
