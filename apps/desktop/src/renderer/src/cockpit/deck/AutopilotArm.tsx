// Autopilot's setup and its guarded switch. MOCK: the boxes are held here, and only the repositories, the instruction and the limits reach Fleet.
// The switch is two acts: the guard flips up, then the red switch flips. Closing the guard on a live switch pushes it off, as the hardware does.

import { useRef, useSyncExternalStore, type KeyboardEvent, type ReactNode } from "react";
import { Lock, Plane } from "lucide-react";
import { Checkbox, Input, Kbd, Tooltip, type AutopilotFields } from "@armada/components";

import { settingsFrom } from "../autopilot";
import { enterAutopilot } from "./store";
import "./autopilot-arm.css";

/** Where the switch stands, one per take of the walk: on the Map's lip, in a bay, or on the setup itself. */
export type ArmSeat = "lip" | "bay" | "setup";
export const ARM_SEAT: ArmSeat = ((seat) => (seat === "bay" || seat === "setup" ? seat : "lip"))(new URLSearchParams(window.location.search).get("take"));

type Arm = { lifted: boolean; on: boolean; repositories: ReadonlySet<string>; work: ReadonlySet<string>; decisions: ReadonlySet<string>; instruction: string; fields: AutopilotFields };

const WORK = [
  { id: "backlog", label: "Work the backlog" },
  { id: "running", label: "Jobs already running" },
];
const DECISIONS = [
  { id: "recommended", label: "Question with a recommended answer" },
  { id: "open", label: "Question with no recommended answer" },
  { id: "plan", label: "Plan to approve" },
  { id: "text", label: "Text to OK" },
  { id: "unsure", label: "Issue Autopilot was unsure of" },
  { id: "walk", label: "Walk of a visible change" },
  { id: "land", label: "Landing a green pull request" },
  { id: "permission", label: "Permission an agent asks for" },
];

let arm: Arm = {
  lifted: false,
  on: false,
  repositories: new Set(),
  work: new Set(["backlog", "running"]),
  decisions: new Set(["recommended", "plan", "text", "land"]),
  instruction: "",
  fields: { atOnce: "2", until: "", spend: "", sessions: "" },
};
const heard = new Set<() => void>();
const move = (change: Partial<Arm>) => {
  arm = { ...arm, ...change };
  heard.forEach((one) => one());
};
const useArm = () =>
  useSyncExternalStore(
    (hear) => (heard.add(hear), () => void heard.delete(hear)),
    () => arm,
  );
const flipped = (set: ReadonlySet<string>, id: string) => {
  const next = new Set(set);
  if (!next.delete(id)) next.add(id);
  return next;
};

function start() {
  const read = settingsFrom(arm.instruction.trim() === "" ? "Work through the open issues" : arm.instruction, arm.fields, Date.now());
  if (!read.ok) return;
  for (const manifest of arm.repositories) void window.armada.startAutopilot?.(manifest, read.settings);
}
function stop() {
  for (const manifest of arm.repositories) void window.armada.stopAutopilot?.(manifest);
}

const ready = (one: Arm) => one.repositories.size > 0;
const guardOf = (from: HTMLElement | null) => from?.closest(".gs")?.querySelector<HTMLElement>(".gs__guard");

/** The hardware. */
export function GuardSwitch({ seat }: { seat: ArmSeat }) {
  const held = useArm();
  const lever = useRef<HTMLButtonElement>(null);
  const lift = () => {
    enterAutopilot();
    if (held.lifted) {
      if (held.on) stop();
      return move({ lifted: false, on: false });
    }
    move({ lifted: true });
    window.requestAnimationFrame(() => lever.current?.focus());
  };
  const flip = () => {
    if (!held.lifted) return;
    if (held.on) {
      stop();
      move({ on: false });
      // The switch is down, then the guard falls on it, and the keys go back to the guard.
      window.setTimeout(() => (move({ lifted: false }), guardOf(lever.current)?.focus()), 180);
      return;
    }
    if (!ready(held)) return;
    start();
    move({ on: true });
  };
  return (
    <section className="gs deck-autopilot" aria-label="Autopilot switch" data-seat={seat} data-lifted={held.lifted || undefined} data-on={held.on || undefined}>
      <span className="gs__screw" data-at="tl" />
      <span className="gs__screw" data-at="tr" />
      <span className="gs__screw" data-at="bl" />
      <span className="gs__screw" data-at="br" />
      <span className="gs__stencil" aria-hidden="true">
        Autopilot
      </span>
      <span className="gs__lamps" aria-hidden="true">
        <span className="gs__lamp" data-lamp="arm" />
        <span className="gs__lamp" data-lamp="on" />
      </span>
      <div className="gs__well">
        <span className="gs__mark" data-mark="on" aria-hidden="true">
          On
        </span>
        <span className="gs__mark" data-mark="off" aria-hidden="true">
          Off
        </span>
        <span className="gs__collar" aria-hidden="true" />
        <Tooltip label={!held.lifted ? "Lift the guard first" : held.on ? "Autopilot on" : ready(held) ? "Flip to start" : "No repository checked"} asChild>
          <button
            ref={lever}
            type="button"
            role="switch"
            aria-checked={held.on}
            aria-label="Autopilot"
            aria-disabled={!held.lifted || (!held.on && !ready(held))}
            tabIndex={held.lifted ? 0 : -1}
            className="gs__bat"
            onClick={flip}
            onKeyDown={(event) => {
              if (event.key !== "Escape" || held.on) return;
              event.stopPropagation();
              move({ lifted: false });
              guardOf(lever.current)?.focus();
            }}
          >
            <span className="gs__lever" aria-hidden="true" />
            <span className="gs__tip" aria-hidden="true" />
          </button>
        </Tooltip>
      </div>
      <Tooltip label={held.lifted ? (held.on ? "Close the guard: Autopilot off" : "Close the guard") : "Lift the guard"} asChild>
        <button type="button" className="gs__guard" aria-label={held.lifted ? "Close guard" : "Lift guard"} aria-pressed={held.lifted} onClick={lift}>
          <span className="gs__tab" aria-hidden="true" />
        </button>
      </Tooltip>
      <span className="gs__hinge" aria-hidden="true" />
    </section>
  );
}

/** The console's key that opens the setup, where the switch stands on the setup itself. */
export function ArmKey() {
  const held = useArm();
  return (
    <Tooltip label="Autopilot" asChild>
      <button type="button" className="gs-key" aria-label="Autopilot" data-on={held.on || undefined} onClick={enterAutopilot}>
        <Plane size={16} aria-hidden="true" />
      </button>
    </Tooltip>
  );
}

/** Up and Down go between the boxes, Space checks, Enter goes to the guard. */
function keys(event: KeyboardEvent<HTMLElement>) {
  const target = event.target as HTMLElement;
  if (event.key === "Enter" && target.matches("input")) {
    event.preventDefault();
    const hardware = document.querySelector<HTMLElement>(".gs");
    return (hardware?.hasAttribute("data-lifted") ? hardware.querySelector<HTMLElement>(".gs__bat") : hardware?.querySelector<HTMLElement>(".gs__guard"))?.focus();
  }
  if ((event.key !== "ArrowDown" && event.key !== "ArrowUp") || !target.matches('input[type="checkbox"]')) return;
  const boxes = [...event.currentTarget.querySelectorAll<HTMLInputElement>('input[type="checkbox"]:not(:disabled)')];
  const at = boxes.indexOf(target as HTMLInputElement);
  event.preventDefault();
  boxes[Math.min(boxes.length - 1, Math.max(0, at + (event.key === "ArrowDown" ? 1 : -1)))]?.focus();
}

/** The setup, on the stage: which repositories, what goes on autopilot, and for each kind of decision who takes it. */
export function AutopilotSetup({ repositories, sidings }: { repositories: readonly { id: string; name: string }[]; sidings: ReactNode }) {
  const held = useArm();
  const field = (key: keyof AutopilotFields, label: string, unit?: string) => <Input label={label} mono value={held.fields[key]} {...(unit === undefined ? {} : { unit })} onChange={(event) => move({ fields: { ...held.fields, [key]: event.currentTarget.value } })} />;
  return (
    <div className="ap-setup" data-seat={ARM_SEAT} onKeyDown={keys}>
      <div className="ap-setup__columns">
        <div className="ap-setup__column">
          <section className="ap-setup__group" aria-label="Repositories">
            <h3 className="ap-setup__name">Repositories</h3>
            {repositories.map((one) => (
              <Checkbox key={one.id} checked={held.repositories.has(one.id)} onChange={() => move({ repositories: flipped(held.repositories, one.id) })}>
                {one.name}
              </Checkbox>
            ))}
          </section>
          <section className="ap-setup__group" aria-label="On autopilot">
            <h3 className="ap-setup__name">On autopilot</h3>
            {WORK.map((one) => (
              <Checkbox key={one.id} checked={held.work.has(one.id)} onChange={() => move({ work: flipped(held.work, one.id) })}>
                {one.label}
              </Checkbox>
            ))}
          </section>
          <section className="ap-setup__group" aria-label="Limits">
            <h3 className="ap-setup__name">Limits</h3>
            <Input label="Instruction" value={held.instruction} onChange={(event) => move({ instruction: event.currentTarget.value })} />
            <div className="ap-setup__limits">
              {field("atOnce", "At once")}
              {field("until", "Until")}
              {field("spend", "Spend", "$")}
              {field("sessions", "Jobs")}
            </div>
          </section>
        </div>
        <section className="ap-setup__group ap-setup__decisions" aria-label="Decisions">
          <h3 className="ap-setup__name">Co-commander decides</h3>
          {DECISIONS.map((one) => {
            const on = held.decisions.has(one.id);
            return (
              <div key={one.id} className="ap-setup__row">
                <Checkbox checked={on} onChange={() => move({ decisions: flipped(held.decisions, one.id) })}>
                  {one.label}
                </Checkbox>
                <span className="ap-setup__who" data-on={on || undefined}>
                  {on ? "Co-commander" : "Blocks"}
                </span>
              </div>
            );
          })}
          <div className="ap-setup__row" data-locked="">
            <Checkbox checked={false} disabled>
              Anything destructive or irreversible
            </Checkbox>
            <Tooltip label="Always blocks until you are back">
              <span className="ap-setup__who">
                <Lock size={12} aria-hidden="true" />
                Blocks
              </span>
            </Tooltip>
          </div>
        </section>
        {ARM_SEAT === "setup" ? (
          <div className="ap-setup__hardware">
            <GuardSwitch seat="setup" />
          </div>
        ) : null}
      </div>
      <p className="ap-setup__foot">
        <Kbd>↵</Kbd> Guard
      </p>
      {sidings}
    </div>
  );
}
