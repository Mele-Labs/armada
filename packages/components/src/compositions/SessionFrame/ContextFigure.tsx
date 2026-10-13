import type { CSSProperties } from "react";

import type { ContextBreakdown } from "../../context-breakdown";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { usageLevel } from "../../usage";
import { StopCard, StopCardList, useCardOpen } from "../StopCard/StopCard";

/** How full the Session's context window is, as it last reported, and what it is made of where that is known. */
export type ContextFill = { tokens: number; window: number; breakdown?: ContextBreakdown };

/** 53840 as "53.8k", 5000 as "5.0k", 142000 as "142k", 1000000 as "1M". */
export function tokensSaid(n: number): string {
  if (n >= 1_000_000) return `${Number((n / 1_000_000).toFixed(1))}M`;
  if (n < 1_000) return String(n);
  const k = n / 1_000;
  return k >= 100 ? `${Math.round(k)}k` : k >= 10 ? `${Number(k.toFixed(1))}k` : `${k.toFixed(1)}k`;
}

const TOP = 3;

/** The heaviest few of a list, by tokens. */
const heaviest = <T extends { tokens: number }>(items: readonly T[]): T[] => [...items].sort((a, b) => b.tokens - a.tokens).slice(0, TOP);

/** A path's last segment, `NOTES.md` for `apps/NOTES.md`; the last two where another file shares it. */
const nameOf = (path: string, all: readonly { path: string }[]): string => {
  const parts = path.split("/");
  const last = parts[parts.length - 1] ?? path;
  return all.filter((one) => one.path.split("/").pop() === last).length > 1 ? parts.slice(-2).join("/") : last;
};

/** The MCP servers by what their tools weigh, heaviest first. */
function heaviestServers(tools: ContextBreakdown["mcpTools"]): { name: string; tokens: number }[] {
  const by = new Map<string, number>();
  for (const tool of tools) by.set(tool.server, (by.get(tool.server) ?? 0) + tool.tokens);
  return heaviest([...by].map(([name, tokens]) => ({ name, tokens })));
}

/** The window as one slim row: a segment for each category in it, the room left, and the autocompact buffer. */
function WindowBar({ breakdown }: { breakdown: ContextBreakdown }) {
  const shown = breakdown.categories.filter((one) => one.kind !== "deferred" && one.tokens > 0);
  return (
    <span className="armada-context-bar" aria-hidden>
      {shown.map((one) => (
        <i key={one.name} data-kind={one.kind} style={{ flexGrow: one.tokens } as CSSProperties} />
      ))}
    </span>
  );
}

function BreakdownCard({ fill, percent, breakdown, pinned }: { fill: ContextFill; percent: number; breakdown: ContextBreakdown; pinned: boolean }) {
  return (
    <StopCard
      className="armada-session-frame__context-card"
      role="dialog"
      aria-label="Context"
      data-pinned={pinned || undefined}
      pill={`${percent}% used`}
      where={`${tokensSaid(fill.tokens)} / ${tokensSaid(fill.window)}`}
      title="Context"
      lead={<WindowBar breakdown={breakdown} />}
      rows={breakdown.categories.filter((one) => one.kind !== "deferred").map((one) => [one.name, tokensSaid(one.tokens), true] as const)}
    >
      <StopCardList heading="Memory files" rows={heaviest(breakdown.memoryFiles.map((one) => ({ name: nameOf(one.path, breakdown.memoryFiles), tokens: one.tokens }))).map((one) => [one.name, tokensSaid(one.tokens), true] as const)} />
      <StopCardList heading="MCP tools" rows={heaviestServers(breakdown.mcpTools).map((one) => [one.name, tokensSaid(one.tokens), true] as const)} />
    </StopCard>
  );
}

/**
 * The live tokens and the percent of the window they fill, in mono and coloured by how full. Where the
 * breakdown is known it is a button: **hover or focus opens the card, a press keeps it open**, and Esc or a
 * press outside closes it. Nothing is drawn without a window to fill.
 */
export function ContextFigure({ fill }: { fill: ContextFill }) {
  const card = useCardOpen();
  if (fill.window <= 0) return null;
  const percent = Math.round((fill.tokens / fill.window) * 100);
  const figure = `${tokensSaid(fill.tokens)} · ${percent}%`;
  if (fill.breakdown === undefined) {
    return (
      <Tooltip label={`Context · ${tokensSaid(fill.tokens)} of ${tokensSaid(fill.window)} tokens`}>
        <span className="armada-session-frame__context" role="img" aria-label={`Context ${percent}%`} data-level={usageLevel(percent)}>
          {figure}
        </span>
      </Tooltip>
    );
  }
  return (
    <span className="armada-session-frame__context-host" data-level={usageLevel(percent)} {...card.bind}>
      <button type="button" className="armada-session-frame__context" aria-label={`Context ${percent}%`} aria-expanded={card.open} onClick={card.toggle}>
        {figure}
      </button>
      {card.open ? <BreakdownCard fill={fill} percent={percent} breakdown={fill.breakdown} pinned={card.pinned} /> : null}
    </span>
  );
}
