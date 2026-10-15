// A Job's route, drawn on the mission's plane: a dotted line across the ground, and on it a pin for each step, standing upright. A
// finished step is a green pin on a lit disc, the step at work an orange one, a step still to come a grey one, a failed one red with
// a flare, and one waiting on him amber with a beacon pinging from the ground. Pressing a pin opens the step's card; hovering names
// it. The battlefield draws the rest: the plots, the drones, the base and the flag. Ported from proto/cockpit-flight-deck. #2301.

import { CircleCheck, CircleDashed, CircleHelp, CircleX, LoaderCircle, type LucideIcon } from "lucide-react";

import { Sprite, keyed, onPlane, px } from "./bf-parts";
import type { OnTip } from "./Drone";
import type { PinIn, PinState, RouteIn } from "./mission";
import type { Tip, TipHue } from "./tip";

const SAID: Record<PinState, string> = { done: "Done", live: "At work", next: "Not started", failed: "Failed", ask: "Waiting for you" };
const GLYPH: Record<PinState, LucideIcon> = { done: CircleCheck, live: LoaderCircle, next: CircleDashed, failed: CircleX, ask: CircleHelp };
const HUE: Record<PinState, TipHue> = { done: "good", live: "calm", next: "quiet", failed: "bad", ask: "ask" };
const TONE: Record<PinState, "calm" | "ask" | "bad" | "good"> = { done: "good", live: "calm", next: "calm", failed: "bad", ask: "ask" };

/** What a pin says when it is hovered. */
export const tipOf = (pin: PinIn): Tip => ({
  label: pin.fan ? "Group" : "Step",
  title: pin.label,
  hue: HUE[pin.state],
  state: { Glyph: GLYPH[pin.state], said: pin.said === "" ? SAID[pin.state] : pin.said, tone: TONE[pin.state] },
});

/** The line a leg draws between two points of the ground: a shallow bow, so a run of legs reads as a path and not a ruler. */
const legOf = (from: { x: number; y: number }, to: { x: number; y: number }) => {
  const [a, b] = [onPlane(from), onPlane(to)];
  const bow = Math.min(18, Math.hypot(b.x - a.x, b.y - a.y) * 0.08);
  const [mx, my] = [(a.x + b.x) / 2, (a.y + b.y) / 2 - bow];
  return `M${a.x.toFixed(1)},${a.y.toFixed(1)} Q${mx.toFixed(1)},${my.toFixed(1)} ${b.x.toFixed(1)},${b.y.toFixed(1)}`;
};

/** The route's legs, lying on the ground: dotted, green where the Job has been over them and grey ahead, a dashed accent where a gate sends work back. */
export function RouteLegs({ route }: { route: RouteIn }) {
  return (
    <g className="bf-ways" aria-hidden="true">
      {route.legs.map((leg) => (
        <path key={leg.id} className="bf-way" data-done={leg.done || undefined} data-back={leg.back || undefined} d={legOf(leg.from, leg.to)} />
      ))}
    </g>
  );
}

/** The mark inside a pin's head. */
function Mark({ state }: { state: PinState }) {
  if (state === "done") return <path className="bf-pin__mark" d="M-5,-26 L-1.5,-22.5 L5,-30" />;
  if (state === "failed") return <path className="bf-pin__mark" d="M-4.5,-29.5 L4.5,-20.5 M4.5,-29.5 L-4.5,-20.5" />;
  if (state === "ask") return <path className="bf-pin__mark" d="M0,-31 V-25 M0,-21.4 V-21.2" />;
  if (state === "live") return <circle className="bf-pin__core" cy={-25.5} r={4} />;
  return <circle className="bf-pin__mark" cy={-25.5} r={3.6} fill="none" />;
}

/** One pin per waypoint, standing on the ground at its place. */
export function Pins({ route, pinned, readable, onPress, onTip }: { route: RouteIn; pinned: string | undefined; readable: boolean; onPress: (pin: PinIn, target: Element) => void; onTip: OnTip }) {
  return (
    <>
      {route.pins.map((pin) => {
        const at = onPlane(pin.at);
        const tip = tipOf(pin);
        const press = (target: Element) => onPress(pin, target);
        return (
          <div
            key={pin.id}
            className="bf-pin"
            role="option"
            tabIndex={0}
            aria-selected={pinned === pin.id}
            aria-label={`${pin.label}, ${pin.said === "" ? SAID[pin.state] : pin.said}`}
            data-pin={pin.id}
            data-label={pin.label}
            data-state={pin.state}
            data-fan={pin.fan || undefined}
            data-here={pin.here || undefined}
            data-pinned={pinned === pin.id || undefined}
            style={{ transform: `translate3d(${px(at.x)}, ${px(at.y)}, ${px(0)})` }}
            onClick={(event) => press(event.currentTarget)}
            onKeyDown={(event) => keyed(event, () => press(event.currentTarget))}
            onMouseEnter={(event) => onTip(tip, event.currentTarget)}
            onMouseLeave={() => onTip(undefined)}
            onFocus={(event) => onTip(tip, event.currentTarget)}
            onBlur={() => onTip(undefined)}
          >
            <div className="bf-pin__ground" aria-hidden="true">
              {pin.state === "ask" ? (
                <>
                  <i className="bf-pin__ping" />
                  <i className="bf-pin__ping" data-late />
                </>
              ) : null}
              <i className="bf-pin__disc" />
            </div>
            <Sprite x={-26} y={-66} w={52} h={72}>
              {pin.state === "failed" ? (
                <g className="bf-pin__flare">
                  <path d="M-9,-44 L-13,-55 M0,-47 L0,-61 M9,-44 L13,-55" />
                </g>
              ) : null}
              <path className="bf-pin__body" d="M0,0 C-3,-8 -12,-15 -12,-25 A12,12 0 0 1 12,-25 C12,-15 3,-8 0,0 Z" />
              <Mark state={pin.state} />
              <rect className="bf-hit" x={-17} y={-44} width={34} height={50} />
            </Sprite>
            {readable ? (
              <Sprite x={-90} y={28} w={180} h={20} label>
                <text className="bf-pin__name" y={42} textAnchor="middle">
                  {pin.label.length > 22 ? `${pin.label.slice(0, 21)}…` : pin.label}
                </text>
              </Sprite>
            ) : null}
          </div>
        );
      })}
    </>
  );
}
