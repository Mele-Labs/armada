import type { CSSProperties } from "react";

import type { ContextBreakdown } from "../../context-breakdown";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { usageLevel } from "../../usage";
import { Layers } from "lucide-react";
import { Inspect, InspectCard, InspectFacts, InspectSection, type InspectFact } from "../../primitives/Inspect/Inspect";

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
function WindowBar({ breakdown, tone }: { breakdown: ContextBreakdown; tone: string }) {
  const shown = breakdown.categories.filter((one) => one.kind !== "deferred" && one.tokens > 0);
  return (
    <span className="armada-context-bar" aria-hidden style={{ "--tone": tone } as CSSProperties}>
      {shown.map((one) => (
        <i key={one.name} data-kind={one.kind} style={{ flexGrow: one.tokens } as CSSProperties} />
      ))}
    </span>
  );
}

const LEVEL_TONE: Record<string, string> = { warning: "var(--status-awaiting-review)", danger: "var(--status-completed-failed)" };

function BreakdownCard({ fill, percent, breakdown }: { fill: ContextFill; percent: number; breakdown: ContextBreakdown }) {
  const tone = LEVEL_TONE[usageLevel(percent)];
  const rows = (list: readonly (readonly [string, number])[]): InspectFact[] => list.map(([label, tokens]) => ({ label, value: tokensSaid(tokens), mono: true }));
  return (
    <InspectCard
      kind={{ Glyph: Layers, label: "Context" }}
      where={`${tokensSaid(fill.tokens)} / ${tokensSaid(fill.window)}`}
      title="Context window"
      state={{ said: `${percent}% used`, ...(tone === undefined ? {} : { tone }) }}
      keys={[{ key: "Enter", verb: "Keep open" }]}
      width="wide"
    >
      <WindowBar breakdown={breakdown} tone={tone ?? "var(--fg-muted)"} />
      <InspectFacts facts={rows(breakdown.categories.filter((one) => one.kind !== "deferred").map((one) => [one.name, one.tokens] as const))} />
      <InspectSection heading="Memory files" facts={rows(heaviest(breakdown.memoryFiles.map((one) => ({ name: nameOf(one.path, breakdown.memoryFiles), tokens: one.tokens }))).map((one) => [one.name, one.tokens] as const))} />
      <InspectSection heading="MCP tools" facts={rows(heaviestServers(breakdown.mcpTools).map((one) => [one.name, one.tokens] as const))} />
    </InspectCard>
  );
}

/** The threshold as a share of the window, where there is one to mark. */
function thresholdOf(fill: ContextFill): number | undefined {
  const at = fill.breakdown?.autocompactThreshold;
  return at === undefined || fill.window <= 0 || at <= 0 || at >= fill.window ? undefined : at / fill.window;
}

/** The custom properties the tank reads: how full, and where the autocompact threshold is. */
function tankStyle(fill: ContextFill): CSSProperties {
  const threshold = thresholdOf(fill);
  return {
    "--fill": `${Math.min(Math.max(fill.tokens / fill.window, 0), 1) * 100}%`,
    ...(threshold === undefined ? {} : { "--threshold": `${threshold * 100}%` }),
  } as CSSProperties;
}

/**
 * The context window as a tank: the live tokens and the percent in mono, on a pill filled to `tokens / window` in the
 * tone of how full it is (normal below 80, warning from 80, danger from 95), with a dotted line at the autocompact
 * threshold where the breakdown holds one. Where the breakdown is known it is a button: **hover or focus opens the
 * card, a press keeps it open**, and Esc or a press outside closes it. Nothing is drawn without a window to fill.
 */
export function ContextFigure({ fill }: { fill: ContextFill }) {
  if (fill.window <= 0) return null;
  const percent = Math.round((fill.tokens / fill.window) * 100);
  const figure = `${tokensSaid(fill.tokens)} · ${percent}%`;
  const threshold = thresholdOf(fill) !== undefined || undefined;
  if (fill.breakdown === undefined) {
    return (
      <Tooltip label={`Context · ${tokensSaid(fill.tokens)} of ${tokensSaid(fill.window)} tokens`}>
        <span className="armada-session-frame__context" role="img" aria-label={`Context ${percent}%`} data-level={usageLevel(percent)} data-threshold={threshold} style={tankStyle(fill)}>
          {figure}
        </span>
      </Tooltip>
    );
  }
  return (
    <span className="armada-session-frame__context-host" data-level={usageLevel(percent)}>
      <Inspect card={<BreakdownCard fill={fill} percent={percent} breakdown={fill.breakdown} />} name="Context">
        <button type="button" className="armada-session-frame__context" aria-label={`Context ${percent}%`} data-threshold={threshold} style={tankStyle(fill)}>
          {figure}
        </button>
      </Inspect>
    </span>
  );
}
