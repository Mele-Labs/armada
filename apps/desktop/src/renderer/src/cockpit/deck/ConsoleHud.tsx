// The console's HUD: real panels projected over the field, the map still drawing behind. One hard-edged frame, Activity and Calls both,
// drawn in pixels so the hairlines stay on the pixel grid. Not tilted: a transform would soften the words and put cards out of place, so what says
// "projected" is the beam down to the facet it comes from.

import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
  type RefObject,
} from "react";
import { createPortal } from "react-dom";
import { X } from "lucide-react";
import { Button, Tooltip } from "@armada/components";
import { holdsText } from "@armada/screens/src/keys";
import { popupIsOpen } from "@armada/screens/src/list-keyboard";

import { keyPressed } from "./flight-controls";

type Box = {
  left: number;
  top: number;
  width: number;
  height: number;
};
export type { Box };
const boxOf = (element: Element): Box => {
  const { left, top, width, height } = element.getBoundingClientRect();
  return {
    left,
    top,
    width,
    height,
  };
};

/** The plot's place in the panel: the scales in its margins, the name over it. */
const PAD = {
  left: 48,
  right: 48,
  top: 40,
  bottom: 40,
};
/** The graph lays an hour across its plot less this much at each end (`Swarm`). */
const MARGIN = 6;
const MINUTES = 60;
/** On the pixel grid, where a one pixel line is sharp. */
const on = (at: number) => Math.round(at) + 0.5;

/** The frame every HUD wears: thin glass, a hairline and proud corners. `head` is the strip the name stands in. */
function Frame({
  w,
  h,
  head,
  children,
}: {
  w: number;
  h: number;
  head: number;
  children?: ReactNode;
}) {
  const arm = 24;
  const out = 4;
  return (
    <svg className="console-hud__frame" width={w} height={h} aria-hidden="true">
      <rect className="console-hud__glass" width={w} height={h} />
      <rect className="console-hud__strip" width={w} height={head} />
      <rect
        className="console-hud__line console-hud__edge"
        x={0.5}
        y={0.5}
        width={w - 1}
        height={h - 1}
      />
      {children}
      <path
        className="console-hud__line console-hud__bracket"
        d={`M${-out} ${arm}V${-out}H${arm}M${w - arm} ${-out}H${w + out}V${arm}M${w + out} ${h - arm}V${h + out}H${w - arm}M${arm} ${h + out}H${-out}V${h - arm}`}
      />
    </svg>
  );
}

/** Activity's scales and graticule in a panel `w` by `h`. Only the time axis reads: up and down is how the dots pack. */
function Graticule({ w, h }: { w: number; h: number }) {
  const left = PAD.left;
  const top = PAD.top;
  const right = w - PAD.right;
  const bottom = h - PAD.bottom;
  const x = (ago: number) => on(left + MARGIN + (1 - ago / MINUTES) * (right - left - MARGIN * 2));
  const y = (share: number) => on(top + share * (bottom - top));
  const minutes = Array.from(
    {
      length: MINUTES + 1,
    },
    (_, ago) => ago,
  );
  const rows = Array.from(
    {
      length: 41,
    },
    (_, at) => at,
  );
  const majors = minutes.filter((ago) => ago % 10 === 0);
  const across = (list: readonly number[]) =>
    list.map((ago) => `M${x(ago)} ${top}V${bottom}`).join("");
  const down = (list: readonly number[]) =>
    list.map((at) => `M${left} ${y(at / 8)}H${right}`).join("");
  return (
    <>
      <rect className="console-hud__strip" y={bottom} width={w} height={h - bottom} />
      <path
        className="console-hud__line console-hud__minor"
        d={
          across(minutes.filter((ago) => ago % 5 === 0 && ago % 10 !== 0)) +
          down([
            1,
            3,
            5,
            7,
          ])
        }
      />
      <path
        className="console-hud__line console-hud__major"
        d={
          across(majors) +
          down([
            2,
            6,
          ])
        }
      />
      <path
        className="console-hud__line console-hud__centre"
        d={down([
          4,
        ])}
      />
      <path
        className="console-hud__line console-hud__edge"
        d={`M${on(left)} ${on(top)}H${on(right)}V${on(bottom)}H${on(left)}Z`}
      />
      <path
        className="console-hud__line console-hud__cross"
        d={majors
          .flatMap((ago) =>
            [
              2,
              4,
              6,
            ].map((at) => `M${x(ago) - 4} ${y(at / 8)}h8M${x(ago)} ${y(at / 8) - 4}v8`),
          )
          .join("")}
      />
      {/* The time axis: a tick a minute, longer at five and ten. */}
      <path
        className="console-hud__line console-hud__tick"
        d={minutes
          .map(
            (ago) =>
              `M${x(ago)} ${on(bottom)}v${ago % 5 === 0 ? 8 : 4}M${x(ago)} ${on(top)}v-${ago % 5 === 0 ? 6 : 3}`,
          )
          .join("")}
      />
      <path
        className="console-hud__line console-hud__tick-major"
        d={majors.map((ago) => `M${x(ago)} ${on(bottom)}v12`).join("")}
      />
      {majors.map((ago) => (
        <text key={ago} className="console-hud__at" x={x(ago)} y={bottom + 26}>
          {ago === 0 ? "0" : `${ago}m`}
        </text>
      ))}
      {/* The side scales, up and down the same from the centre line. */}
      <path
        className="console-hud__line console-hud__tick"
        d={rows
          .map(
            (at) =>
              `M${on(left)} ${y(at / 40)}h-${at % 5 === 0 ? 12 : 6}M${on(right)} ${y(at / 40)}h${at % 5 === 0 ? 12 : 6}`,
          )
          .join("")}
      />
      <path
        className="console-hud__line console-hud__tick-major"
        d={`M${on(left)} ${y(0.5)}h-20M${on(right)} ${y(0.5)}h20`}
      />
      <path className="console-hud__line console-hud__now" d={`M${x(0)} ${top}V${bottom}`} />
      <path className="console-hud__now-mark" d={`M${x(0) - 5} ${top - 8}h10l-5 8z`} />
    </>
  );
}

/** Where the panel stands over the stage: the upper part of the field, leaving the rest of it open. */
export const panelOf = (stage: Box): Box => {
  const inset = Math.min(72, stage.width * 0.07);
  return {
    left: Math.round(stage.left + inset),
    top: Math.round(stage.top + 24),
    width: Math.round(stage.width - inset * 2),
    height: Math.round(Math.max(280, stage.height * 0.62)),
  };
};

/** The beam comes up from the facet and lands on the panel's foot as near over it as the panel reaches. */
const beamOf = (panel: Box, facet: Box) => {
  const foot = panel.top + panel.height + 4;
  const wide = Math.min(facet.width * 1.25, panel.width / 2);
  const left = Math.max(
    panel.left + 24,
    Math.min(panel.left + panel.width - 24 - wide, facet.left + (facet.width - wide) / 2),
  );
  return {
    foot,
    left,
    right: left + wide,
  };
};

const STOPS =
  "button:not([tabindex='-1']):not(:disabled), a[href], input, textarea, select, [role='textbox'], [tabindex='0']";
/** The box he is typing in, if it holds anything: Esc leaves that to him. */
const typedIn = (target: EventTarget | null): boolean => {
  if (!(target instanceof HTMLElement) || !holdsText(target)) return false;
  return (
    (target instanceof HTMLTextAreaElement || target instanceof HTMLInputElement
      ? target.value
      : (target.textContent ?? "")
    ).trim() !== ""
  );
};

type HudProps = {
  /** What the person reads the HUD as, in the dialog's name. */
  label: string;
  /** The words in the head, where they differ from `label`; nothing where there is nothing to name. */
  name?: string | undefined;
  /** Presses in the head, before Close. */
  acts?: ReactNode;
  /** What the beam comes up from. */
  facet: RefObject<Element | null>;
  onClose: () => void;
  /** Its place over the stage. */
  place: (stage: Box) => Box;
  /** What else is drawn in the frame, in the panel's pixels. */
  art?: (w: number, h: number) => ReactNode;
  /** The head strip's height, Activity's reaching the top of its plot. */
  head: number;
  /** A key of the HUD's own, heard before anything inside it. True where it took the key. */
  onKey?: (event: KeyboardEvent) => boolean;
  /** Where focus lands on opening: the panel, unless the HUD lands it itself. */
  landsItself?: boolean;
  /** Set on the panel so its own styles can find it. */
  kind: string;
  /** A dialog, unless what stands in it must keep the page's keys: the thread's own stand down for a dialog. */
  role?: "dialog" | "region";
  /** What stands in the panel, given its size. */
  children: (w: number, h: number) => ReactNode;
};

/** The HUD, over the stage with the field still drawing behind it. Esc and the facet's own press close it; Tab goes round inside it while it is open. */
export function ConsoleHud({
  label,
  name,
  acts,
  facet,
  onClose,
  place,
  art,
  head,
  onKey,
  landsItself,
  kind,
  role = "dialog",
  children,
}: HudProps) {
  const [at, setAt] = useState<
    | {
        stage: Box;
        facet: Box | undefined;
      }
    | undefined
  >();
  const panel = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const stage = document.querySelector(".deck-stage");
    const from = facet.current;
    if (stage === null) return;
    // A key moved out of the bays leaves nothing to beam from, and the panel stands with no beam.
    const measure = () =>
      setAt({
        stage: boxOf(stage),
        facet: from === null ? undefined : boxOf(from),
      });
    measure();
    const watch = new ResizeObserver(measure);
    watch.observe(stage);
    if (from !== null) watch.observe(from);
    window.addEventListener("resize", measure);
    return () => {
      watch.disconnect();
      window.removeEventListener("resize", measure);
    };
  }, [
    facet,
  ]);
  const placed = at !== undefined;
  useEffect(() => {
    if (placed && landsItself !== true) panel.current?.focus();
  }, [
    placed,
    landsItself,
  ]);
  useEffect(() => {
    const key = (event: KeyboardEvent) => {
      // A menu or a confirm open over the HUD keeps its own keys.
      if (
        popupIsOpen() ||
        [
          ...document.querySelectorAll('[role="dialog"], [role="alertdialog"]'),
        ].some((one) => one !== panel.current)
      )
        return;
      if (event.key === "Escape") {
        // Taken already is the press that went back from a Job and brought this HUD up.
        if (event.defaultPrevented) return;
        event.stopPropagation();
        event.preventDefault();
        // Words typed stay: the first Esc only leaves the box.
        if (typedIn(event.target)) panel.current?.focus();
        else onClose();
        return;
      }
      // The deck stands down while a HUD is up, so the facet's own key is heard here: it closes what it opened. Taken already is the press that opened it.
      const own = facet.current instanceof HTMLElement ? facet.current.dataset.key : undefined;
      if (own !== undefined && !event.defaultPrevented && keyPressed(own, event)) {
        event.stopPropagation();
        event.preventDefault();
        onClose();
        return;
      }
      if (onKey?.(event) === true) return;
      if (event.key !== "Tab" || panel.current === null) return;
      const stops = [
        ...panel.current.querySelectorAll<HTMLElement>(STOPS),
      ].filter((one) => one.checkVisibility());
      const was = stops.indexOf(document.activeElement as HTMLElement);
      event.preventDefault();
      const next =
        was === -1
          ? event.shiftKey
            ? stops.length - 1
            : 0
          : (was + (event.shiftKey ? -1 : 1) + stops.length) % stops.length;
      stops[next]?.focus();
    };
    document.addEventListener("keydown", key, true);
    return () => document.removeEventListener("keydown", key, true);
  }, [
    facet,
    onClose,
    onKey,
  ]);
  if (at === undefined) return null;
  const box = place(at.stage);
  const beam = at.facet === undefined ? undefined : beamOf(box, at.facet);
  return createPortal(
    <div className="console-hud" data-hud={kind}>
      {beam === undefined || at.facet === undefined ? null : (
        <svg className="console-hud__beam" aria-hidden="true">
          <path
            className="console-hud__beam-fill"
            d={`M${beam.left} ${beam.foot}H${beam.right}L${at.facet.left + at.facet.width} ${at.facet.top}H${at.facet.left}Z`}
          />
          <path
            className="console-hud__beam-edge"
            d={`M${beam.left} ${beam.foot}L${at.facet.left} ${at.facet.top}M${beam.right} ${beam.foot}L${at.facet.left + at.facet.width} ${at.facet.top}`}
          />
          <path
            className="console-hud__beam-collar"
            d={`M${beam.left} ${beam.foot}H${beam.right}`}
          />
        </svg>
      )}
      <div
        ref={panel}
        className="console-hud__panel"
        role={role}
        aria-label={label}
        tabIndex={-1}
        style={box}
      >
        <Frame w={box.width} h={box.height} head={head}>
          {art?.(box.width, box.height)}
        </Frame>
        {name === undefined ? null : <h2 className="console-hud__name">{name}</h2>}
        {children(box.width, box.height)}
        <span className="console-hud__acts">
          {acts}
          <Tooltip label="Close" shortcut="Esc" asChild>
            <Button variant="ghost" size="sm" iconOnly aria-label="Close" onClick={onClose}>
              <X size={16} aria-hidden="true" />
            </Button>
          </Tooltip>
        </span>
      </div>
    </div>,
    document.body,
  );
}

/** Activity over the field: the real graph, large, on its scales. */
export function ActivityHud({
  graph,
  facet,
  onClose,
}: {
  graph: ReactNode;
  facet: RefObject<Element | null>;
  onClose: () => void;
}) {
  return (
    <ConsoleHud
      label="Activity"
      name="Activity"
      facet={facet}
      onClose={onClose}
      place={panelOf}
      head={PAD.top}
      kind="activity"
      art={(w, h) => <Graticule w={w} h={h} />}
    >
      {(w, h) => (
        <div
          className="console-hud__plot"
          style={{
            left: PAD.left,
            top: PAD.top,
            width: w - PAD.left - PAD.right,
            height: h - PAD.top - PAD.bottom,
          }}
        >
          {graph}
        </div>
      )}
    </ConsoleHud>
  );
}
