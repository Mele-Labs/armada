// Autopilot's setup and its guarded switch. MOCK: the boxes are held here, and only the repositories, the instruction and the limits reach Fleet.
// The switch is two acts: the guard flips up, then the red switch flips. Closing the guard on a live switch pushes it off, as the hardware does.

import { useEffect, useRef, useSyncExternalStore, type KeyboardEvent, type ReactNode } from "react";
import { Lock, Plane } from "lucide-react";
import { Checkbox, Input, Textarea, Tooltip, useTimeFormat, type AutopilotFields } from "@armada/components";

import { settingsFrom } from "../autopilot";
import { ConsoleHud, type Box } from "./ConsoleHud";
import { deckNow, select } from "./store";
import "./autopilot-arm.css";

/** Where the switch stands, one per take of the walk: on the Map's lip, in a bay, or on the setup itself. */
export type ArmSeat = "lip" | "bay" | "setup";
export const ARM_SEAT: ArmSeat = ((seat) => (seat === "bay" || seat === "setup" ? seat : "lip"))(new URLSearchParams(window.location.search).get("take"));

type Arm = { open: boolean; lifted: boolean; on: boolean; repositories: ReadonlySet<string>; work: ReadonlySet<string>; decisions: ReadonlySet<string>; instruction: string; fields: AutopilotFields };

const WORK = [
  { id: "backlog", label: "Work the backlog" },
  { id: "running", label: "Jobs already running" },
];
type Decision = { id: string; label: string; tip?: string; locked?: string };
const DECISIONS: readonly { name: string; items: readonly Decision[] }[] = [
  {
    name: "Questions",
    items: [
      { id: "recommended", label: "With a recommended answer" },
      { id: "open", label: "With no recommended answer" },
    ],
  },
  {
    name: "Hand-offs",
    items: [
      { id: "plan", label: "Plan to approve" },
      { id: "draft", label: "Draft to send", tip: "Words a Job wrote that need your OK, such as a pull request description, a comment or a message" },
      { id: "walk", label: "Walk of a visible change" },
      { id: "land", label: "Landing a green pull request" },
    ],
  },
  {
    name: "Work",
    items: [
      { id: "unsure", label: "Issue Autopilot was unsure of" },
      { id: "red", label: "Fix a red main" },
    ],
  },
  {
    name: "Permissions",
    items: [
      { id: "reads", label: "Reads" },
      { id: "writes", label: "Writes" },
      { id: "destructive", label: "Destructive", locked: "Always blocks until you are back" },
    ],
  },
];

let arm: Arm = {
  open: false,
  lifted: false,
  on: false,
  repositories: new Set(),
  work: new Set(["backlog", "running"]),
  decisions: new Set(["recommended", "plan", "draft", "land", "red", "reads"]),
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
/** What the HUD's beam comes up from: the switch on the console, or the console's key where the switch stands in the HUD. */
const facetEl = () => document.querySelector<HTMLElement>(ARM_SEAT === "setup" ? ".gs-key" : ".console-bay > .gs");

/** The console only stands over the field, so a Job open on the stage gives way to it. */
export function openSetup() {
  if (deckNow().sel !== undefined) select(undefined);
  window.requestAnimationFrame(() => {
    move({ open: true });
    window.requestAnimationFrame(() => document.querySelector<HTMLElement>(".ap-hud input")?.focus());
  });
}
/** A guard left up on a switch that is not lit falls with the setup. */
function closeSetup() {
  if (!arm.open) return;
  move({ open: false, lifted: arm.on });
  document.querySelector<HTMLElement>(ARM_SEAT === "setup" ? ".gs-key" : ".console-bay > .gs .gs__guard")?.focus();
}

/** The hardware. */
export function GuardSwitch({ seat }: { seat: ArmSeat }) {
  const held = useArm();
  const plate = useRef<HTMLElement>(null);
  const focusOn = (part: string) => plate.current?.querySelector<HTMLElement>(part)?.focus();
  const lift = () => {
    if (held.lifted) {
      if (held.on) stop();
      return move({ lifted: false, on: false, open: seat === "setup" });
    }
    move({ lifted: true, open: true });
    window.requestAnimationFrame(() => focusOn(".gs__bat"));
  };
  const flip = () => {
    if (!held.lifted) return;
    if (held.on) {
      stop();
      move({ on: false });
      // The switch is down, then the guard falls on it, and the keys go back to the guard.
      window.setTimeout(() => (move({ lifted: false }), focusOn(".gs__guard")), 180);
      return;
    }
    if (!ready(held)) return;
    start();
    move({ on: true });
  };
  return (
    <section ref={plate} className="gs deck-autopilot" aria-label={seat === "setup" ? "Setup switch" : "Autopilot switch"} data-seat={seat} data-lifted={held.lifted || undefined} data-on={held.on || undefined}>
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
            type="button"
            role="switch"
            aria-checked={held.on}
            aria-label="Autopilot"
            aria-disabled={!held.lifted || (!held.on && !ready(held))}
            tabIndex={held.lifted ? 0 : -1}
            className="gs__bat"
            onClick={flip}
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
      <button type="button" className="gs-key" aria-label="Autopilot" data-on={held.on || undefined} onClick={openSetup}>
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
    const hardware = document.querySelector<HTMLElement>(".ap-hud .gs");
    return (hardware?.hasAttribute("data-lifted") ? hardware.querySelector<HTMLElement>(".gs__bat") : hardware?.querySelector<HTMLElement>(".gs__guard"))?.focus();
  }
  if ((event.key !== "ArrowDown" && event.key !== "ArrowUp") || !target.matches('input[type="checkbox"]')) return;
  const boxes = [...event.currentTarget.querySelectorAll<HTMLInputElement>('input[type="checkbox"]:not(:disabled)')];
  const at = boxes.indexOf(target as HTMLInputElement);
  event.preventDefault();
  boxes[Math.min(boxes.length - 1, Math.max(0, at + (event.key === "ArrowDown" ? 1 : -1)))]?.focus();
}

const HEAD = 44;
/** Where the columns part, in the panel's pixels. Too narrow for three, the first two stack. */
const partsOf = (w: number) => (w < 720 ? [Math.round(w * 0.46)] : [Math.round(w * 0.24), Math.round(w * 0.53)]);
const on = (at: number) => Math.round(at) + 0.5;

/** The hairlines between the columns, each a scale: a tick every 8 pixels, longer every 40. */
function Parts({ w, h }: { w: number; h: number }) {
  const parts = partsOf(w).map(on);
  const ticks = Array.from({ length: Math.floor((h - HEAD) / 8) }, (_, at) => at);
  return (
    <>
      <path className="console-hud__line console-hud__edge" d={`M0 ${on(HEAD)}H${w}`} />
      <path className="console-hud__line console-hud__major" d={parts.map((x) => `M${x} ${HEAD}V${h}`).join("")} />
      <path className="console-hud__line console-hud__tick" d={parts.flatMap((x) => ticks.map((at) => `M${x} ${on(HEAD + at * 8)}h${at % 5 === 0 ? 8 : 4}`)).join("")} />
      <path className="console-hud__line console-hud__cross" d={parts.map((x) => `M${x - 4} ${on(HEAD)}h8M${x} ${HEAD - 4}v8`).join("")} />
    </>
  );
}

/** All the stage's height it needs, for every decision to stand whole: the brackets clear of the title bar above and the beam's collar clear of the console below. */
const placeOf = (stage: Box): Box => {
  const inset = stage.width < 800 ? 0 : Math.min(56, stage.width * 0.05);
  return { left: Math.round(stage.left + inset), top: Math.round(stage.top + 6), width: Math.round(stage.width - inset * 2), height: Math.round(Math.min(stage.height - 14, 560)) };
};

const Group = ({ name, children }: { name: string; children: ReactNode }) => (
  <section className="ap-hud__group" aria-label={name}>
    <h3 className="ap-hud__head">{name}</h3>
    {children}
  </section>
);

function DecisionRow({ one, checked }: { one: Decision; checked: boolean }) {
  const box = (
    <Checkbox checked={checked} disabled={one.locked !== undefined} onChange={() => move({ decisions: flipped(arm.decisions, one.id) })}>
      {one.label}
    </Checkbox>
  );
  return (
    <div className="ap-hud__row" data-locked={one.locked === undefined ? undefined : ""}>
      {one.tip === undefined ? box : <Tooltip label={one.tip}>{box}</Tooltip>}
      {one.locked === undefined ? (
        <span className="ap-hud__who" data-on={checked || undefined}>
          {checked ? "Co-commander" : "Blocks"}
        </span>
      ) : (
        <Tooltip label={one.locked}>
          <span className="ap-hud__who">
            <Lock size={12} aria-hidden="true" />
            Blocks
          </span>
        </Tooltip>
      )}
    </div>
  );
}

/** The setup, a HUD over the field, with a switch of its own that moves with the console's: which repositories, what goes on autopilot, its limits, and for each kind of decision who takes it. */
export function ArmSetup({ repositories, sidings }: { repositories: readonly { id: string; name: string }[]; sidings: ReactNode }) {
  const held = useArm();
  const { hour12 } = useTimeFormat();
  const facet = useRef<Element | null>(null);
  facet.current = held.open ? facetEl() : null;
  // Going into a Job puts the HUD away with the console it stood over.
  useEffect(() => {
    if (held.open && facetEl() === null) closeSetup();
  });
  // The arm is kept outside this component, and the next Cockpit drawn must not open on it.
  useEffect(() => () => closeSetup(), []);
  if (!held.open || facet.current === null) return null;
  const field = (key: keyof AutopilotFields, label: string, extra?: { unit?: string; placeholder?: string }) => <Input label={label} mono value={held.fields[key]} {...extra} onChange={(event) => move({ fields: { ...held.fields, [key]: event.currentTarget.value } })} />;
  return (
    <ConsoleHud label="Autopilot setup" name="Autopilot" facet={facet} onClose={closeSetup} place={placeOf} head={HEAD} landsItself kind="autopilot" role="region" art={(w, h) => <Parts w={w} h={h} />}>
      {(w) => {
        const parts = partsOf(w);
        return (
          <div className="ap-hud" data-parts={parts.length} style={{ top: HEAD, gridTemplateColumns: parts.length === 1 ? `${parts[0]}px 1fr` : `${parts[0]}px ${parts[1]! - parts[0]!}px 1fr` }} onKeyDown={keys}>
            <div className="ap-hud__column">
              <Group name="Repositories">
                {repositories.map((one) => (
                  <Checkbox key={one.id} checked={held.repositories.has(one.id)} onChange={() => move({ repositories: flipped(held.repositories, one.id) })}>
                    {one.name}
                  </Checkbox>
                ))}
              </Group>
              <Group name="On autopilot">
                {WORK.map((one) => (
                  <Checkbox key={one.id} checked={held.work.has(one.id)} onChange={() => move({ work: flipped(held.work, one.id) })}>
                    {one.label}
                  </Checkbox>
                ))}
              </Group>
              <div className="ap-hud__hardware">
                <GuardSwitch seat="setup" />
              </div>
            </div>
            <div className="ap-hud__column">
              <Group name="Instruction">
                <Textarea aria-label="Instruction" rows={5} keep="autopilot:instruction" value={held.instruction} onChange={(event) => move({ instruction: event.currentTarget.value })} />
              </Group>
              <Group name="Limits">
                <div className="ap-hud__limits">
                  {field("atOnce", "Jobs at a time")}
                  {field("until", "Stop at", { placeholder: hour12 ? "7:00 AM" : "07:00" })}
                  {field("spend", "Spend up to", { unit: "$" })}
                  {field("sessions", "Jobs in total")}
                </div>
              </Group>
              {sidings === undefined ? null : <div className="ap-hud__sidings">{sidings}</div>}
            </div>
            <section className="ap-hud__column ap-hud__decisions" aria-label="Decisions">
              {DECISIONS.map((group) => (
                <Group key={group.name} name={group.name}>
                  {group.items.map((one) => (
                    <DecisionRow key={one.id} one={one} checked={held.decisions.has(one.id)} />
                  ))}
                </Group>
              ))}
            </section>
          </div>
        );
      }}
    </ConsoleHud>
  );
}
