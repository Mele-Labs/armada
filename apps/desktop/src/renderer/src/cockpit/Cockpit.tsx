// Command Central as a cockpit: the viewscreen shows every live Job and Session at a glance, and a
// call that needs the owner comes forward over it, centred and answerable from the keyboard. Calls
// behind it wait in the band at the edge; `l` puts the one in front back there. Nothing here is a
// list to read down: the glass is what is running, the card is what is wanted. Mock only.

import { useEffect, useMemo, useRef, useState } from "react";
import { Activity, Bot, Box, Clock, Cpu, GitMerge, LoaderCircle, ScanLine, Scale, ShieldCheck, SquareTerminal, Waypoints, Workflow, type LucideIcon } from "lucide-react";
import { Button, Kbd, Tooltip, actionOf, keyFor } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import type { CallView } from "@armada/jobs/draft/calls";
import type { NowView } from "@armada/jobs/draft/now";
import { nowPanelOf } from "@armada/jobs";
import { isTerminal } from "@armada/screens";
import { holdsText } from "@armada/screens/src/keys";
import { useListKeydown } from "@armada/screens/src/list-keyboard";

import type { BridgeState } from "../../../shared/bridge";
import { age, useBoardKeys, useCursor, useItems, type Hosts, type Item } from "../Dashboard";
import { viewsOf } from "../merge-line";
import { useSessions } from "../sessions-draft";
import { CallCard } from "./CallCard";
import type { Answering } from "./answers";
import { instrumentsOf, type Instrument, type Standing } from "./instruments";
import { TAB_KEYS } from "./keys";
import "./cockpit.css";

const GLYPH: Record<Standing, LucideIcon> = {
  asking: SquareTerminal,
  running: LoaderCircle,
  queued: Clock,
  proposing: ScanLine,
  working: SquareTerminal,
  idle: SquareTerminal,
};

/** What is running, by its kind: the same three marks the Now panel draws. */
const KIND: Record<"drone" | "check" | "judge", { Glyph: LucideIcon; said: string }> = {
  drone: { Glyph: Bot, said: "Drone" },
  check: { Glyph: ShieldCheck, said: "Check" },
  judge: { Glyph: Scale, said: "Judge" },
};

/** Why nothing runs, by its kind: the Now panel's marks again. */
const WAIT: Record<"resource" | "job" | "transition" | "step", { Glyph: LucideIcon; said: string }> = {
  resource: { Glyph: Cpu, said: "Resource" },
  job: { Glyph: Box, said: "Job" },
  transition: { Glyph: Waypoints, said: "Transition" },
  step: { Glyph: Workflow, said: "Step" },
};

/** Whether motion is off: nothing waits for an exit that will not play. */
const stillness = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/** The merge line as the glass's horizon: blocks moving toward main's light. */
function Horizon({ state }: { state: BridgeState }) {
  const views = viewsOf(state);
  if (views.length === 0) return null;
  return (
    <>
      {views.map((view) => (
        <div key={view.root} className="armada-view__line" aria-label="Merge line">
          <GitMerge size={16} aria-hidden="true" />
          <ol className="armada-view__belt">
            {view.line.map((entry) => (
              <Tooltip key={entry.branch} label={`${entry.branch}, ${entry.state}`} asChild>
                <li className="armada-view__block" data-state={entry.state} />
              </Tooltip>
            ))}
          </ol>
          <Tooltip label={view.hub?.main?.state === "red" ? "Main is red" : "Main is green"}>
            <span className="armada-view__main" data-red={view.hub?.main?.state === "red" || undefined} />
          </Tooltip>
        </div>
      ))}
    </>
  );
}

/** One live Job or Session: the glyph for how it stands, a gauge of its workflow, what it is doing. */
function InstrumentTile({ one, now, selected, hosts, onPick }: { one: Instrument; now: number; selected: boolean; hosts: Hosts; onPick: () => void }) {
  const Mark = one.call?.icon ?? GLYPH[one.standing];
  const open = () => (one.job === undefined ? hosts.onOpenSession(one.key.slice("session:".length)) : hosts.onOpen(one.job.id));
  const stoppable = one.job !== undefined && !isTerminal(one.job) && hosts.onKill !== undefined;
  return (
    <li
      id={`inst-${one.key}`}
      role="option"
      aria-selected={selected}
      className="armada-inst"
      data-hue={one.hue}
      data-standing={one.standing}
      data-job-id={one.job?.id}
      data-status={one.job?.status}
      onClick={onPick}
    >
      <div className="armada-inst__band">
        <Tooltip label={one.named}>
          <span className="armada-inst__mark" role="img" aria-label={one.named}>
            <Mark size={16} aria-hidden="true" />
          </span>
        </Tooltip>
        <span className="armada-inst__where">{one.where}</span>
        <span className="armada-inst__age">{age(one.since, now)}</span>
      </div>
      <div className="armada-inst__body">
        <span className="armada-inst__title">{one.title}</span>
        {one.doing === undefined ? null : (
          <span className="armada-inst__doing" data-state={one.doing.state}>
            <Tooltip label={KIND[one.doing.of].said}>
              <span role="img" aria-label={KIND[one.doing.of].said}>
                {(() => {
                  const { Glyph } = KIND[one.doing.of];
                  return <Glyph size={12} aria-hidden="true" />;
                })()}
              </span>
            </Tooltip>
            {one.doing.name}
          </span>
        )}
        {one.doing !== undefined || one.waiting === undefined ? null : (
          <span className="armada-inst__doing" data-waiting>
            <Tooltip label={WAIT[one.waiting.kind].said}>
              <span role="img" aria-label={WAIT[one.waiting.kind].said}>
                {(() => {
                  const { Glyph } = WAIT[one.waiting.kind];
                  return <Glyph size={12} aria-hidden="true" />;
                })()}
              </span>
            </Tooltip>
            {one.waiting.text}
          </span>
        )}
        {one.line === undefined ? null : <span className="armada-inst__line">{one.line}</span>}
      </div>
      {one.steps.length === 0 ? null : (
        <ol className="armada-inst__gauge" aria-label={`${one.title}, steps`}>
          {one.steps.map((step, index) => (
            <Tooltip key={step.id} label={step.label} asChild>
              <li className="armada-inst__seg" data-seg={index < one.at ? "done" : index === one.at ? "here" : "ahead"} />
            </Tooltip>
          ))}
        </ol>
      )}
      {selected ? (
        <div className="armada-inst__acts">
          <Button variant="ghost" size="sm" onClick={open}>
            {actionOf("open").verb}
            <Kbd>↵</Kbd>
          </Button>
          {stoppable ? (
            <Button variant="ghost" size="sm" onClick={() => hosts.onKill!(one.job!.id)}>
              {actionOf("kill").verb}
              <Kbd>{keyFor("kill")}</Kbd>
            </Button>
          ) : null}
        </div>
      ) : null}
    </li>
  );
}

export function Cockpit({
  state,
  now,
  picked,
  nowViews,
  nows,
  ...hosts
}: Hosts & {
  state: BridgeState;
  now: number;
  picked: RepositorySummary | null;
  nowViews?: Readonly<Record<string, CallView>> | undefined;
  nows?: Readonly<Record<string, NowView>> | undefined;
}) {
  const [answered, setAnswered] = useState<ReadonlySet<string>>(new Set());
  const calls = useItems("command-central", state, picked, nowViews, hosts, answered);
  const sessions = useSessions();
  const instruments = useMemo(() => instrumentsOf(state, picked, nowViews, nows, calls, sessions), [state, picked, nowViews, nows, calls, sessions]);

  // Calls put off for later, in the order they were; and one brought back to the front by choice.
  const [put, setPut] = useState<readonly string[]>([]);
  const [forward, setForward] = useState<string>();
  const [leaving, setLeaving] = useState<"later" | "sent">();
  const held = new Set(calls.map((one) => one.key));
  const waiting = put.filter((key) => key !== forward && held.has(key));
  const ahead = calls.filter((one) => !waiting.includes(one.key));
  const front = ahead.find((one) => one.key === forward) ?? ahead[0];
  const edge: Item[] = [...ahead.filter((one) => one !== front), ...waiting.map((key) => calls.find((one) => one.key === key)!)];

  /** Keys pressed while a card is leaving, said again once it has gone. */
  const typed = useRef<KeyboardEventInit[]>([]);
  /** Plays the card's exit, then applies what it was leaving for. */
  const leave = (how: "later" | "sent", apply: () => void) => {
    if (stillness()) return apply();
    setLeaving(how);
    window.setTimeout(() => {
      setLeaving(undefined);
      apply();
      const again = typed.current.splice(0);
      window.setTimeout(() => again.forEach((init) => document.body.dispatchEvent(new KeyboardEvent("keydown", { ...init, bubbles: true, cancelable: true }))), 50);
    }, 200);
  };
  const finish = () =>
    front === undefined
      ? undefined
      : leave("sent", () => {
          setAnswered((was) => new Set([...was, front.key]));
          setForward(undefined);
        });
  const later = () =>
    front === undefined
      ? undefined
      : leave("later", () => {
          setPut((was) => [...was.filter((key) => key !== front.key), front.key]);
          setForward(undefined);
        });
  const recall = (key: string) => {
    setPut((was) => was.filter((one) => one !== key));
    setForward(key);
  };

  // The cursor on the glass.
  const [selected, setSelected] = useState<string>();
  const at = Math.max(0, instruments.findIndex((one) => one.key === selected));
  const current = instruments[at];
  useCursor(current?.job, hosts.onCursor);
  useEffect(() => {
    document.getElementById(`inst-${current?.key}`)?.scrollIntoView({ block: "nearest" });
  }, [current?.key]);
  const grid = useRef<HTMLOListElement>(null);

  // A call arriving takes the keys: a field that was holding them, empty, lets go.
  useEffect(() => {
    const field = document.activeElement;
    if (front !== undefined && field instanceof HTMLInputElement && field.value === "") field.blur();
  }, [front?.key]);

  // j, k, Enter, o and x are the Board's, as every list reads them; the rest are this screen's.
  useBoardKeys(instruments, at, setSelected, hosts, front === undefined && leaving === undefined);
  const answering = useRef<Answering | undefined>(undefined);
  useListKeydown((event) => {
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    const moves = ["j", "k", "ArrowDown", "ArrowUp", "ArrowLeft", "ArrowRight"].includes(event.key);
    if (event.repeat && !moves) return;
    if (holdsText(event.target)) {
      // The dispatch bar is always empty: Escape or Down hands the keys back to the glass.
      const field = event.target;
      if (field instanceof HTMLInputElement && field.value === "" && (event.key === "Escape" || event.key === "ArrowDown")) {
        event.preventDefault();
        field.blur();
      }
      return;
    }
    if (leaving !== undefined) {
      // Enter is not said again: a second press must never send the next call.
      if (event.key !== "Enter") typed.current.push({ key: event.key, code: event.code, shiftKey: event.shiftKey });
      return;
    }
    const claim = () => event.preventDefault();

    if (front !== undefined) {
      const card = answering.current;
      if (card === undefined) return;
      if (/^[1-9]$/.test(event.key)) return claim(), card.pick(Number(event.key) - 1);
      switch (event.key) {
        case "ArrowDown":
        case "j":
          return claim(), card.step(1);
        case "ArrowUp":
        case "k":
          return claim(), card.step(-1);
        case "Enter":
          // A focused button fires itself on Enter.
          if ((event.target as HTMLElement).tagName === "BUTTON") return;
          return card.canSend ? (claim(), card.send()) : undefined;
        case "l":
        case "Escape":
          return claim(), later();
        case "o":
          return card.open === undefined ? undefined : (claim(), card.open());
      }
      return;
    }

    const cols = grid.current === null ? 1 : Math.max(1, getComputedStyle(grid.current).gridTemplateColumns.split(" ").length);
    const to = (by: number) => {
      const next = instruments[Math.min(instruments.length - 1, Math.max(0, at + by))];
      if (next !== undefined) setSelected(next.key);
    };
    switch (event.key) {
      case "ArrowRight":
        return claim(), to(1);
      case "ArrowLeft":
        return claim(), to(-1);
      case "ArrowDown":
        return claim(), to(cols);
      case "ArrowUp":
        return claim(), to(-cols);
      case "w":
        return edge[0] === undefined ? undefined : (claim(), recall(edge[0].key));
    }
  });

  const arrived = useRef(new Set<string>());
  const frame = front?.hue ?? edge[0]?.hue;

  return (
    <div className="armada-cockpit" data-hue={frame} data-settles>
      <section className="armada-view" aria-label="Command Central" data-hue={frame} data-waiting={frame === undefined ? undefined : ""}>
        <header className="armada-view__band">
          <Tooltip label="Fleet">
            <Activity size={16} aria-label="Fleet" />
          </Tooltip>
          <Horizon state={state} />
          {edge.length === 0 ? null : (
            <ul className="armada-dock" aria-label="Waiting calls">
              {edge.map((one, index) => {
                const Icon = one.icon;
                const fresh = !arrived.current.has(one.key);
                arrived.current.add(one.key);
                return (
                  <li key={one.key} data-hue={one.hue}>
                    <Tooltip label={`${one.kind}: ${one.title}`}>
                      <button type="button" className="armada-dock__chip" data-hue={one.hue} data-deferred={waiting.includes(one.key) || undefined} data-arrived={fresh || undefined} aria-label={`${one.kind}: ${one.title}`} onClick={() => recall(one.key)}>
                        <Icon size={12} aria-hidden="true" />
                        <span>{one.title}</span>
                        {index === 0 ? <Kbd>w</Kbd> : null}
                      </button>
                    </Tooltip>
                  </li>
                );
              })}
            </ul>
          )}
          <span className="armada-view__keys">
            <Tooltip label={actionOf("move_focus").verb}>
              <span>
                <Kbd>j</Kbd> <Kbd>k</Kbd>
              </span>
            </Tooltip>
            <Tooltip label="Previous and next tab">
              <span>
                <Kbd>{TAB_KEYS.previous}</Kbd> <Kbd>{TAB_KEYS.next}</Kbd>
              </span>
            </Tooltip>
            <Tooltip label="Keys">
              <Kbd>?</Kbd>
            </Tooltip>
          </span>
        </header>
        <div className="armada-view__stage">
          <ol ref={grid} className="armada-view__grid" role="listbox" aria-label="Running" aria-activedescendant={current === undefined ? undefined : `inst-${current.key}`} tabIndex={0} data-recessed={front === undefined || leaving !== undefined ? undefined : ""}>
            {instruments.map((one) => (
              <InstrumentTile key={one.key} one={one} now={now} selected={one.key === current?.key} hosts={hosts} onPick={() => setSelected(one.key)} />
            ))}
          </ol>
          {front === undefined ? null : (
            <div className="armada-cockpit__scrim" data-hue={front.hue}>
              <CallCard key={front.key} item={front} now={now} nowing={front.job === undefined ? undefined : nowPanelOf(nows?.[front.job.id], { onOpenJob: hosts.onOpen, onSaid: () => {} })} state={state} hosts={hosts} finish={finish} later={later} leaving={leaving} answering={answering} />
            </div>
          )}
        </div>
      </section>
    </div>
  );
}
