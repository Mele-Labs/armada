import { CircleDot } from "lucide-react";
import { useEffect, useRef, useState, type KeyboardEvent } from "react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { StepActivityMark } from "../StepActivityMark/StepActivityMark";

/** A Check the Session has out: its run on the Checks page, its key, what it was narrowed to, and the clock time it started. */
export type RunningCheck = { run: number; name: string; target?: string; at: string };

/** "14:23:10" or "14:23" as seconds into a day; undefined for anything else. */
function clockSeconds(at: string): number | undefined {
  const [h, m, s] = at.split(":").map(Number);
  if (h === undefined || m === undefined || Number.isNaN(h) || Number.isNaN(m) || Number.isNaN(s ?? 0)) return undefined;
  return h * 3600 + m * 60 + (s ?? 0);
}

/** How long a Check has run, from the clock time it started: "42s", "3m 12s", "1h 5m". A start after now is read as yesterday's. */
export function ranFor(at: string, now: Date): string | undefined {
  const start = clockSeconds(at);
  if (start === undefined) return undefined;
  const here = now.getHours() * 3600 + now.getMinutes() * 60 + now.getSeconds();
  const took = (here - start + 86_400) % 86_400;
  if (took < 60) return `${took}s`;
  if (took < 3600) return `${Math.floor(took / 60)}m ${took % 60}s`;
  return `${Math.floor(took / 3600)}h ${Math.floor((took % 3600) / 60)}m`;
}

/** The mark's tooltip and name: "2 checks running". */
const saidOf = (count: number): string => `${count} ${count === 1 ? "check" : "checks"} running`;

/** The list's items, in the order drawn. */
const itemsOf = (panel: HTMLElement | null): HTMLElement[] => [...(panel?.querySelectorAll<HTMLElement>('[role="menuitem"]') ?? [])];

/**
 * The Checks a Session has out, in the header: the running glyph the thread's check rows wear and the count in mono, named
 * by its tooltip. **A press opens a list of them**, each with how long it has run, and a press on one opens that run.
 * ↑ and ↓ walk the list, Enter opens a Check, Esc closes it and returns focus to the mark. Nothing is drawn at none.
 */
export function RunningChecks({ checks, onOpenCheck }: { checks: readonly RunningCheck[]; onOpenCheck?: (run: number) => void }) {
  const [open, setOpen] = useState(false);
  const [now, setNow] = useState(() => new Date());
  const host = useRef<HTMLSpanElement>(null);
  const button = useRef<HTMLButtonElement>(null);
  const panel = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    itemsOf(panel.current)[0]?.focus();
    setNow(new Date());
    const tick = setInterval(() => setNow(new Date()), 1000);
    const outside = (event: PointerEvent) => {
      if (host.current !== null && !host.current.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", outside);
    return () => {
      clearInterval(tick);
      document.removeEventListener("pointerdown", outside);
    };
  }, [open]);

  // The last Check ending leaves nothing to list, so the list goes with the mark.
  useEffect(() => {
    if (checks.length === 0) setOpen(false);
  }, [checks.length]);

  if (checks.length === 0) return null;
  const said = saidOf(checks.length);

  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key === "Escape" && open) {
      event.preventDefault();
      event.stopPropagation();
      setOpen(false);
      button.current?.focus();
      return;
    }
    if (!open || panel.current?.contains(document.activeElement) !== true) return;
    const items = itemsOf(panel.current);
    const at = items.indexOf(document.activeElement as HTMLElement);
    const to = event.key === "ArrowDown" ? (at + 1) % items.length : event.key === "ArrowUp" ? (at <= 0 ? items.length : at) - 1 : event.key === "Home" ? 0 : event.key === "End" ? items.length - 1 : null;
    if (to === null) return;
    event.preventDefault();
    items[to]?.focus();
  };

  return (
    <span ref={host} className="armada-session-frame__checks-host" onKeyDown={onKeyDown}>
      <Tooltip asChild label={said}>
        <button ref={button} type="button" className="armada-session-frame__checks" aria-label={said} aria-haspopup="menu" aria-expanded={open} onClick={() => setOpen((on) => !on)}>
          <span className="armada-session-mark">
            <CircleDot size={16} strokeWidth={2} aria-hidden />
          </span>
          {checks.length}
        </button>
      </Tooltip>
      {open ? (
        <div ref={panel} className="armada-session-frame__checks-list" role="menu" aria-label={said}>
          {checks.map((one) => (
            <button
              key={one.run}
              type="button"
              role="menuitem"
              className="armada-session-frame__checks-item"
              aria-label={`Check ${one.name}${one.target === undefined ? "" : ` ${one.target}`}, running`}
              onClick={() => {
                setOpen(false);
                onOpenCheck?.(one.run);
              }}
            >
              <StepActivityMark activity="running" label="running" pulsing />
              <span className="armada-session-frame__checks-name">{one.name}</span>
              {one.target === undefined ? null : <span className="armada-session-frame__checks-target">{one.target}</span>}
              <span className="armada-session-frame__checks-ran">{ranFor(one.at, now)}</span>
            </button>
          ))}
        </div>
      ) : null}
    </span>
  );
}
