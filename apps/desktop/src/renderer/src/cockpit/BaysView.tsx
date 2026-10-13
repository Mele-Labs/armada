// The Cockpit's Bays filter: the work in every worktree bay, and what waits for one, as metro lines
// through the same stations, Waiting to main (`metro.ts` says where each stands and what each stop says).
// A stop opens its card on hover or focus and keeps it open when pressed; a line's title selects its
// work as the grid and the map do. What moves is what is still working: nothing here does.

import { Fragment, useEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { LoaderCircle, Plane } from "lucide-react";
import { Button, StopCard, Tooltip } from "@armada/components";

import type { BaysRead } from "./bays";
import { MAIN, REVIEW, STATIONS, metroOf, type Act, type Card, type Line, type LineHolder, type Metro, type Tone } from "./metro";

const TONE: Record<Tone, string> = {
  run: "var(--status-running)",
  ask: "var(--status-awaiting-review)",
  bad: "var(--status-completed-failed)",
  ok: "var(--status-completed-success)",
  idle: "var(--status-not-started)",
  esc: "var(--status-escalated)",
};

const LEGEND: readonly (readonly [Tone, string])[] = [
  ["run", "working"],
  ["ask", "needs you"],
  ["bad", "failing"],
  ["ok", "passed"],
  ["idle", "waiting"],
];

const two = (n: number) => String(n).padStart(2, "0");

type Open = { id: string; card: Card; station: string; left: number; top: number; pinned: boolean };

/** The legend's two Autopilot marks, beside the tones while one is on. */
function AutopilotKey() {
  return (
    <>
      <span>
        <Plane size={12} aria-hidden="true" />
        picked by Autopilot
      </span>
      <span>
        <i data-picked />
        picked, not started
      </span>
    </>
  );
}

/** What an Autopilot that is on adds: its marks on the lines, and its sidings under them (`AutopilotSidings.tsx`). */
export type BaysAutopilot = { mark: (metro: Metro) => Metro; sidings: ReactNode };

/** Whose work the pull requests under it are: the Job or Session, its live state as its tile draws it, and a press opens it. */
function Holder({ holder, onOpen }: { holder: LineHolder; onOpen: (act: Act) => void }) {
  const alive = holder.item?.live === true && holder.item.settling !== true;
  const spins = holder.item?.mark === undefined && alive;
  const State = holder.item === undefined ? undefined : (holder.item.mark ?? (spins ? LoaderCircle : holder.item.icon));
  const said = holder.item === undefined ? undefined : (holder.item.state ?? (holder.item.hue === "ask" ? "Waiting on you" : holder.item.kind));
  return (
    <li className="armada-metro__row" data-holder data-wants={holder.tone === "ask" ? "you" : undefined} style={{ ["--line" as string]: TONE[holder.tone] } as CSSProperties}>
      <button type="button" className="armada-metro__who" aria-label={`${holder.kind === "job" ? "Job" : "Session"}: ${holder.title}`} data-opens="" onClick={() => onOpen(holder.open)}>
        <span className="armada-metro__bay">
          {State === undefined || said === undefined ? null : (
            <Tooltip label={said}>
              <span role="img" aria-label={said} className="armada-metro__state" data-spin={spins || undefined} data-live={(!spins && alive) || undefined}>
                <State size={14} aria-hidden="true" />
              </span>
            </Tooltip>
          )}
        </span>
        <span className="armada-metro__mark" data-session={holder.kind === "session" || undefined} />
        <Tooltip label={holder.open.label}>
          <span className="armada-metro__title">{holder.title}</span>
        </Tooltip>
      </button>
    </li>
  );
}

function Row({ line, stations, selected, onSelect, onOpen, onCard, onLeave, nextUp = false }: {
  line: Line;
  /** The stations drawn, each with its place on the line: Review only while some line has one. */
  stations: readonly (readonly [string, number])[];
  selected: boolean;
  onSelect: () => void;
  onOpen: (act: Act) => void;
  onCard: (id: string, card: Card, station: string, at: HTMLElement, pin: boolean) => void;
  onLeave: () => void;
  nextUp?: boolean;
}) {
  return (
    <li className="armada-metro__row" data-wants={line.wants} data-owned={line.holder === undefined ? undefined : ""} data-selected={selected || undefined} style={{ ["--line" as string]: TONE[line.tone] } as CSSProperties}>
      <button type="button" className="armada-metro__who" aria-label={line.from !== undefined ? `Pull request: ${line.title}` : line.bay === undefined ? `Waiting: ${line.title}` : `Bay ${two(line.bay)}: ${line.title}`} aria-pressed={selected}
        title={line.pull !== undefined ? undefined : line.open === undefined ? "A terminal outside Armada holds this bay: nothing to open" : line.open.label}
        data-opens={line.open === undefined ? undefined : ""}
        onClick={() => {
          onSelect();
          if (line.open !== undefined) onOpen(line.open);
        }}
      >
        <span className="armada-metro__bay" data-pull={line.pull === undefined ? undefined : ""}>{line.pull !== undefined ? `#${line.pull.number}` : line.bay === undefined ? "—" : two(line.bay)}</span>
        {line.pull === undefined ? (
          <span className="armada-metro__mark" data-session={line.session || undefined} />
        ) : (
          <Tooltip label={line.pull.ci}>
            <span className="armada-metro__mark" role="img" aria-label={line.pull.ci} />
          </Tooltip>
        )}
        {line.autopilot === true ? <Plane className="armada-metro__autopilot" size={12} aria-label="Picked by Autopilot" /> : null}
        {line.pull === undefined || line.open === undefined ? (
          <span className="armada-metro__title" title={line.title}>
            {line.title}
          </span>
        ) : (
          <Tooltip label={`Open ${line.pull.said}`}>
            <span className="armada-metro__title">{line.title}</span>
          </Tooltip>
        )}
      </button>
      {!nextUp ? null : (
        <span className="armada-metro__station" data-next-up data-state={line.nextUp === undefined ? "ahead" : "now"}>
          {line.nextUp === undefined ? (
            <span className="armada-metro__stop" aria-hidden="true" />
          ) : (
            <button type="button" className="armada-metro__stop" data-picked aria-label={`${line.title}: Next up, ${line.nextUp.card.pill}`} style={{ ["--tone" as string]: TONE[line.nextUp.tone] } as CSSProperties} onMouseEnter={(event) => onCard(`${line.key}:next`, line.nextUp!.card, "Next up", event.currentTarget, false)} onMouseLeave={onLeave} onFocus={(event) => onCard(`${line.key}:next`, line.nextUp!.card, "Next up", event.currentTarget, false)} onClick={(event) => onCard(`${line.key}:next`, line.nextUp!.card, "Next up", event.currentTarget, true)} />
          )}
        </span>
      )}
      {stations.map(([station, at]) => {
        const stop = line.stops[at];
        // Before the station a line starts at, it has no track: a pull request opened outside a bay was never in one.
        const state = at < (line.from ?? 0) ? "none" : at < line.reach ? "past" : at === line.reach ? "now" : "ahead";
        const id = `${line.key}:${at}`;
        return (
          <span key={station} className="armada-metro__station" data-state={state} data-first={at === (line.from ?? 0) || undefined} data-last={at === MAIN || undefined}>
            {state === "none" ? null : stop === undefined || state === "ahead" ? (
              <span className="armada-metro__stop" aria-hidden="true" />
            ) : (
              <button
                type="button"
                className="armada-metro__stop"
                data-session={line.session && at > 0 ? "" : undefined}
                data-dashed={stop.dashed === true ? "" : undefined}
                aria-label={`${line.title}: ${station}, ${stop.card.pill}`}
                style={{ ["--tone" as string]: TONE[stop.tone], ["--p" as string]: stop.progress ?? 100 } as CSSProperties}
                onMouseEnter={(event) => onCard(id, stop.card, station, event.currentTarget, false)}
                onMouseLeave={onLeave}
                onFocus={(event) => onCard(id, stop.card, station, event.currentTarget, false)}
                onClick={(event) => onCard(id, stop.card, station, event.currentTarget, true)}
              >
                {state === "now" && stop.glyph !== undefined ? <b>{stop.glyph}</b> : null}
              </button>
            )}
          </span>
        );
      })}
    </li>
  );
}

export function Bays({ read, reading, now, selected, onSelect, onOpenJob, onOpenSession, onOpenLink, autopilot }: {
  read: BaysRead;
  /** Drawn only while an Autopilot is on: the Next up station, its marks and its sidings. */
  autopilot?: BaysAutopilot | undefined;
  /** Fleet has not answered for the pool yet. */
  reading: boolean;
  now: number;
  /** The key of the tile picked, a Job's or a Session's. */
  selected: string | undefined;
  onSelect: (key: string) => void;
  onOpenJob: (id: string) => void;
  onOpenSession: (id: string) => void;
  onOpenLink: (address: string) => void;
}) {
  const metro = useMemo(() => (autopilot === undefined ? metroOf(read, now) : autopilot.mark(metroOf(read, now))), [read, now, autopilot]);
  const nextUp = autopilot !== undefined;
  const stations = STATIONS.map((station, at): readonly [string, number] => [station, at]).filter(([, at]) => at !== REVIEW || metro.review);
  const host = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState<Open>();

  // A pinned card stays until Escape, a press outside it, or a press on its stop again.
  useEffect(() => {
    if (open?.pinned !== true) return;
    const key = (event: KeyboardEvent) => event.key === "Escape" && setOpen(undefined);
    const press = (event: MouseEvent) => {
      const target = event.target as HTMLElement;
      if (target.closest(".armada-metro__card") === null && target.closest(".armada-metro__stop") === null) setOpen(undefined);
    };
    window.addEventListener("keydown", key);
    window.addEventListener("mousedown", press);
    return () => {
      window.removeEventListener("keydown", key);
      window.removeEventListener("mousedown", press);
    };
  }, [open?.pinned]);

  // A hovered card closes a moment after the pointer leaves its stop, so the pointer can cross to it and
  // press its acts; on the card, it stays.
  const closing = useRef<ReturnType<typeof setTimeout>>(undefined);
  const hold = () => clearTimeout(closing.current);
  const leave = () => {
    hold();
    closing.current = setTimeout(() => setOpen((one) => (one?.pinned === true ? one : undefined)), 250);
  };
  useEffect(() => hold, []);

  const onCard = (id: string, card: Card, station: string, at: HTMLElement, pin: boolean) => {
    hold();
    if (open?.pinned === true && !pin) return;
    if (pin && open?.pinned === true && open.id === id) return setOpen(undefined);
    const base = host.current?.getBoundingClientRect();
    const box = at.getBoundingClientRect();
    if (base === undefined) return;
    const width = 288;
    const left = Math.max(8, Math.min(base.width - width - 8, box.left - base.left + box.width / 2 - width / 2));
    setOpen({ id, card, station, left, top: box.bottom - base.top + 8, pinned: pin });
  };
  // A card that would run past the foot of the view opens above its stop instead.
  const card = useRef<HTMLDivElement>(null);
  const [above, setAbove] = useState(false);
  useEffect(() => {
    const one = card.current;
    const base = host.current;
    if (one === null || base === null || open === undefined) return;
    setAbove(open.top + one.offsetHeight > base.clientHeight - 8);
  }, [open]);

  const act = (one: Act) => {
    setOpen(undefined);
    if (one.kind === "job") onOpenJob(one.id);
    else if (one.kind === "session") onOpenSession(one.id);
    else onOpenLink(one.url);
  };

  if (read.harbours.length === 0) {
    return <p className="armada-metro__empty">{reading ? "Reading the bays…" : "Fleet serves no worktree bays for this repository."}</p>;
  }
  const named = metro.harbours.length > 1;
  return (
    <div ref={host} className="armada-metro" data-next-up={nextUp || undefined} style={{ ["--stations" as string]: stations.length + (nextUp ? 1 : 0) } as CSSProperties} onMouseLeave={leave}>
      <div className="armada-metro__head" role="presentation">
        <span className="armada-metro__label">Work</span>
        {nextUp ? <span className="armada-metro__station-name" data-next-up>Next up</span> : null}
        {stations.map(([station, at]) => (
          <span key={station} className="armada-metro__station-name">
            {station}
            <span className="armada-metro__counts">
              {at === MAIN ? (
                <span style={{ ["--tone" as string]: TONE[metro.main === "red" ? "bad" : "ok"] } as CSSProperties}>{metro.main ?? "—"}</span>
              ) : (
                metro.counts[at]!.map(([tone, n]) => (
                  <span key={tone} style={{ ["--tone" as string]: TONE[tone] } as CSSProperties}>
                    {n}
                  </span>
                ))
              )}
            </span>
          </span>
        ))}
      </div>
      <div className="armada-metro__lines">
        {metro.harbours.map((harbour) => (
          <ol key={harbour.manifest} className="armada-metro__block" aria-label={named ? `${harbour.name} bays` : "Bays"}>
            {named ? <li className="armada-metro__part">{harbour.name}</li> : null}
            {harbour.lines.map((line) => (
              <Row key={line.key} line={line} stations={stations} nextUp={nextUp} selected={line.item !== undefined && line.item.key === selected} onSelect={() => (line.item === undefined ? undefined : onSelect(line.item.key))} onOpen={act} onCard={onCard} onLeave={leave} />
            ))}
          </ol>
        ))}
        {metro.harbours.map((harbour) =>
          harbour.outside.length === 0 ? null : (
            <ol key={`${harbour.manifest}:outside`} className="armada-metro__block" aria-label={named ? `${harbour.name} pull requests outside a bay` : "Pull requests outside a bay"}>
              <li className="armada-metro__part">{named ? `${harbour.name}: pull requests outside a bay` : "Pull requests outside a bay"}</li>
              {harbour.outside.map((line, at) => {
                const before = harbour.outside[at - 1]?.holder;
                return (
                  <Fragment key={line.key}>
                    {line.holder === undefined || (before?.kind === line.holder.kind && before.id === line.holder.id) ? null : <Holder holder={line.holder} onOpen={act} />}
                    <Row line={line} stations={stations} nextUp={nextUp} selected={false} onSelect={() => undefined} onOpen={act} onCard={onCard} onLeave={leave} />
                  </Fragment>
                );
              })}
            </ol>
          ),
        )}
        {(metro.nextUp ?? []).length === 0 ? null : (
          <ol className="armada-metro__block" aria-label="Picked by Autopilot, not started">
            <li className="armada-metro__part">Picked by Autopilot, not started</li>
            {metro.nextUp!.map((line) => (
              <Row key={line.key} line={line} stations={stations} nextUp selected={line.item !== undefined && line.item.key === selected} onSelect={() => (line.item === undefined ? undefined : onSelect(line.item.key))} onOpen={act} onCard={onCard} onLeave={leave} />
            ))}
          </ol>
        )}
        {metro.waiting.length === 0 ? null : (
          <ol className="armada-metro__block" aria-label="Waiting for a bay">
            <li className="armada-metro__part">Waiting for a bay</li>
            {metro.waiting.map((line) => (
              <Row key={line.key} line={line} stations={stations} nextUp={nextUp} selected={line.item !== undefined && line.item.key === selected} onSelect={() => (line.item === undefined ? undefined : onSelect(line.item.key))} onOpen={act} onCard={onCard} onLeave={leave} />
            ))}
          </ol>
        )}
        {autopilot?.sidings}
      </div>
      <footer className="armada-metro__foot">
        <span>Bays</span>
        <span className="armada-metro__strip" role="img" aria-label={`${metro.bays.held} held, ${metro.bays.free} free, ${metro.bays.closed} closed`}>
          {metro.bays.strip.map((kind, at) => (
            <i key={at} data-kind={kind} title={metro.bays.names[at]} />
          ))}
        </span>
        <span>
          {metro.bays.held} held · {metro.bays.free} free · {metro.bays.closed} closed
        </span>
        <span className="armada-metro__legend">
          {LEGEND.map(([tone, said]) => (
            <span key={tone}>
              <i style={{ ["--tone" as string]: TONE[tone] } as CSSProperties} />
              {said}
            </span>
          ))}
          {nextUp ? <AutopilotKey /> : null}
        </span>
      </footer>
      {open === undefined ? null : (
        <StopCard
          ref={card}
          className="armada-metro__card"
          role="dialog"
          aria-label={open.card.title}
          data-pinned={open.pinned || undefined}
          onMouseEnter={hold}
          onMouseLeave={leave}
          style={{ ["--tone" as string]: TONE[open.card.tone], left: open.left, ...(above ? { bottom: (host.current?.clientHeight ?? 0) - open.top + 40 } : { top: open.top }) } as CSSProperties}
          pill={open.card.pill}
          where={open.station}
          title={open.card.title}
          rows={open.card.rows}
        >
          {open.card.acts.length === 0 ? null : (
            <span className="armada-metro__acts">
              {open.card.acts.map((one) => (
                <Button key={one.label} variant="secondary" size="sm" onClick={() => act(one)}>
                  {one.label}
                </Button>
              ))}
            </span>
          )}
          {open.pinned ? null : <span className="armada-metro__hint">Click the stop to keep this open</span>}
        </StopCard>
      )}
    </div>
  );
}
