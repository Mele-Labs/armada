// The autonomy defaults of a level (armada-cwj): one row for each kind of act, how far Armada goes alone, and which level set it. Presentational: the host keeps the layers.

import { RotateCcw, Split } from "lucide-react";
import { useId } from "react";

import { Button } from "../../primitives/Button/Button";
import { Radio } from "../../primitives/Radio/Radio";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { listed } from "../LaneCanvas/model";
import type { Autonomy } from "../LaneCanvas/model";
import { ACTS } from "./RuleForm";

/** The kinds of act a default is set for, as the wire names them. */
export type AutonomyKind = "tracker_write" | "forge_write" | "draft" | "start_job" | "start_watch" | "tell_agent" | "run_command" | "run_agent";

/** What one level sets: a kind absent from it falls through to the level above. */
export type AutonomyLayer = Partial<Record<AutonomyKind, Autonomy>>;

/** What each level sets, by the key the host keeps it under. */
export type AutonomyLayers = Readonly<Record<string, AutonomyLayer>>;

/** A level a page reads: the key its layer is kept under and the name it is set by. */
export type AutonomyLevel = { key: string; name: string };

/** The levels a page reads, nearest first. Levels in one tier are peers, as the Workspaces a repository is in: where they set a kind differently the one that asks more wins and the row says so. */
export type AutonomyTiers = readonly (readonly AutonomyLevel[])[];

const KINDS: readonly { kind: AutonomyKind; said: string }[] = [
  { kind: "tracker_write", said: "Ticket writes" },
  { kind: "forge_write", said: "Pull request writes" },
  { kind: "draft", said: "Drafts" },
  { kind: "start_job", said: "Start a Job" },
  { kind: "start_watch", said: "Watches" },
  { kind: "tell_agent", said: "Tell the agent" },
  { kind: "run_command", said: "Commands" },
  { kind: "run_agent", said: "Agents" },
];

/** What Armada does where no level says: a write to the tracker or the forge, a draft and a new Job ask, and the rest act. */
const ARMADA: Readonly<Record<AutonomyKind, Autonomy>> = {
  tracker_write: "asks",
  forge_write: "asks",
  draft: "asks",
  start_job: "asks",
  start_watch: "acts",
  tell_agent: "acts",
  run_command: "acts",
  run_agent: "acts",
};

const LEVELS: readonly Autonomy[] = ["acts", "asks", "off"];
const SAID: Readonly<Record<Autonomy, string>> = { acts: "Acts", asks: "Asks", off: "Off" };

/** How much a value asks of a person: where peers differ, the one that asks more wins. */
const STRICT: Readonly<Record<Autonomy, number>> = { acts: 0, asks: 1, off: 2 };

export type AutonomyRow = {
  kind: AutonomyKind;
  said: string;
  /** The acts the kind covers, for its tooltip. */
  acts: string;
  value: Autonomy;
  /** The level that set it. */
  setBy: string;
  /** The first level sets it, so it can be reset. */
  own: boolean;
  /** Where peers set it differently: the tooltip, naming them. */
  clash?: string;
  /** What it falls back to: the level above and its value. */
  above: { name: string; value: Autonomy };
};

/** Each kind as the first level reads it: its value, the level that set it, and what pressing reset would leave. */
export function autonomyRows(layers: AutonomyLayers, tiers: AutonomyTiers): AutonomyRow[] {
  const valueFrom = (from: number, kind: AutonomyKind): { value: Autonomy; name: string; clash?: string } => {
    for (const tier of tiers.slice(from)) {
      const set = tier.flatMap((level) => {
        const value = layers[level.key]?.[kind];
        return value === undefined ? [] : [{ level, value }];
      });
      if (set.length === 0) continue;
      const winner = set.reduce((best, one) => (STRICT[one.value] > STRICT[best.value] ? one : best));
      const differ = set.some((one) => one.value !== winner.value);
      return { value: winner.value, name: winner.level.name, ...(differ ? { clash: `${listed(set.map((one) => one.level.name))} differ` } : {}) };
    }
    return { value: ARMADA[kind], name: "Armada" };
  };
  return KINDS.map(({ kind, said }) => {
    const here = valueFrom(0, kind);
    const own = layers[tiers[0]?.[0]?.key ?? ""]?.[kind] !== undefined;
    // The way back names the level above and what it reads, even where that level takes it from further up.
    const above = valueFrom(1, kind);
    const next = tiers[1];
    const nextSets = next?.filter((level) => layers[level.key]?.[kind] !== undefined) ?? [];
    const aboveName = next === undefined ? "Armada" : nextSets.length > 0 ? above.name : (next[0]?.name ?? "Armada");
    return {
      kind,
      said,
      acts: ACTS.filter((one) => one.kind === kind).map((one) => one.said).join(", "),
      value: here.value,
      setBy: here.name,
      own,
      ...(here.clash === undefined || own ? {} : { clash: here.clash }),
      above: { name: aboveName, value: above.value },
    };
  });
}

const COLUMNS = "minmax(0, 1fr) repeat(3, var(--w-autonomy-cell)) var(--w-autonomy-by) var(--h-control-sm) var(--h-control-sm)";

/** The rows under a band that names the columns. A column stays where a row lacks its control, so the clash and reset columns hold their places down the list. */
export function AutonomyPanel({ rows, onSet }: { rows: readonly AutonomyRow[]; onSet: (kind: AutonomyKind, level: Autonomy | undefined) => void }) {
  const name = useId();
  return (
    <div className="armada-wm-step armada-wm-auto">
      <div className="armada-wm-step__band armada-wm-auto__row armada-wm-auto__band" style={{ gridTemplateColumns: COLUMNS }} aria-hidden="true">
        <span />
        {LEVELS.map((one) => (
          <span key={one} className="armada-wm-auto__center">
            {SAID[one]}
          </span>
        ))}
        <span>Set by</span>
        <span />
        <span />
      </div>
      <ul className="armada-wm-auto__rows" aria-label="Autonomy">
        {rows.map((row) => {
          const group = `${name}-${row.kind}`;
          const reset = `Use ${row.above.name}'s: ${SAID[row.above.value]}`;
          return (
            <li key={row.kind} className="armada-wm-auto__row" data-own={row.own ? "" : undefined} style={{ gridTemplateColumns: COLUMNS }}>
              <Tooltip label={row.acts}>
                <span className="armada-wm-auto__kind">{row.said}</span>
              </Tooltip>
              {LEVELS.map((one) => (
                <span key={one} className="armada-wm-auto__center armada-wm-auto__cell">
                  <Radio name={group} aria-label={`${row.said}: ${SAID[one]}`} checked={row.value === one} onChange={() => onSet(row.kind, one)}>
                    <span className="armada-wm-hidden">{SAID[one]}</span>
                  </Radio>
                </span>
              ))}
              <span className="armada-wm-auto__by" data-own={row.own ? "" : undefined}>
                {row.setBy}
              </span>
              {row.clash === undefined ? (
                <span aria-hidden="true" />
              ) : (
                <Tooltip label={row.clash}>
                  <span className="armada-wm-auto__clash" role="img" aria-label={row.clash}>
                    <Split size={16} strokeWidth={2} aria-hidden="true" />
                  </span>
                </Tooltip>
              )}
              {row.own ? (
                <Tooltip label={reset}>
                  <Button variant="ghost" size="sm" iconOnly aria-label={reset} onClick={() => onSet(row.kind, undefined)}>
                    <RotateCcw size={16} strokeWidth={2} aria-hidden />
                  </Button>
                </Tooltip>
              ) : (
                <span aria-hidden="true" />
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
