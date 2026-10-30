// The autonomy defaults for a Runs in tab (armada-cwj, mock takes): one row for each kind of act, how far Armada goes alone, and which level set it. Presentational: the host keeps the layers.

import { RotateCcw } from "lucide-react";
import { useId } from "react";

import { Button } from "../../primitives/Button/Button";
import { Radio } from "../../primitives/Radio/Radio";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { DEFAULT } from "../LaneCanvas/model";
import type { Autonomy } from "../LaneCanvas/model";
import { ACTS } from "./RuleForm";

/** The kinds of act a default is set for, as the wire names them. */
export type AutonomyKind = "tracker_write" | "forge_write" | "draft" | "start_job" | "start_watch" | "tell_agent" | "run_command" | "run_agent";

/** The three ways the rows are drawn. `table` has a radio column for each answer, `segmented` one segmented control for each row, `cycle` one button for each row that moves to the next answer. */
export type AutonomyTake = "table" | "segmented" | "cycle";

/** What one level sets: a kind absent from it falls through to the level above. */
export type AutonomyLayer = Partial<Record<AutonomyKind, Autonomy>>;

export type AutonomyEditing = {
  take: AutonomyTake;
  /** What each level sets, by the id of the tab that sets it: the default's tab is Everywhere's, `workspace` is the Workspace's, and a repository's tab is its id. */
  layers: Readonly<Record<string, AutonomyLayer>>;
  /** One tab sets a kind, or clears it so that it falls through to the level above. */
  onSet: (tab: string, kind: AutonomyKind, level: Autonomy | undefined) => void;
};

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

/** A level a tab reads: the id its layer is kept under and the name it is set by. */
export type AutonomyLevel = { id: string; name: string };

/** The levels a tab reads, nearest first, as the editor knows them: the tab's own, the Workspace's where there is one, then Everywhere's. */
export const autonomyLevels = (scope: string, names: { workspace: string | undefined; repository: string | undefined }): AutonomyLevel[] => {
  const everywhere = { id: DEFAULT, name: "Everywhere" };
  if (scope === DEFAULT) return [everywhere];
  const workspace = names.workspace === undefined ? [] : [{ id: "workspace", name: names.workspace }];
  return scope === "workspace" ? [...workspace, everywhere] : [{ id: scope, name: names.repository ?? scope }, ...workspace, everywhere];
};

type Row = {
  kind: AutonomyKind;
  said: string;
  /** The acts the kind covers, for its tooltip. */
  acts: string;
  value: Autonomy;
  /** The level that set it. */
  setBy: string;
  /** This tab sets it, so it can be reset. */
  own: boolean;
  /** What it falls back to: the level above and its value. */
  above: { name: string; value: Autonomy };
};

/** Each kind as the tab reads it: its value, the level that set it, and what pressing reset would leave. */
export function autonomyRows(layers: AutonomyEditing["layers"], levels: readonly AutonomyLevel[]): Row[] {
  const valueFrom = (from: number, kind: AutonomyKind): { value: Autonomy; name: string } => {
    for (const level of levels.slice(from)) {
      const set = layers[level.id]?.[kind];
      if (set !== undefined) return { value: set, name: level.name };
    }
    return { value: ARMADA[kind], name: "Armada" };
  };
  return KINDS.map(({ kind, said }) => {
    const here = valueFrom(0, kind);
    const own = levels[0] !== undefined && layers[levels[0].id]?.[kind] !== undefined;
    // The way back names the level above and what it reads, even where that level takes it from further up.
    const above = valueFrom(1, kind);
    return {
      kind,
      said,
      acts: ACTS.filter((one) => one.kind === kind).map((one) => one.said).join(", "),
      value: here.value,
      setBy: here.name,
      own,
      above: { name: levels[1]?.name ?? "Armada", value: above.value },
    };
  });
}

const COLUMNS: Readonly<Record<AutonomyTake, string>> = {
  table: "minmax(0, 1fr) repeat(3, var(--w-autonomy-cell)) var(--w-autonomy-by) var(--h-control-sm)",
  segmented: "minmax(0, 1fr) var(--w-autonomy-segments) var(--w-autonomy-by) var(--h-control-sm)",
  cycle: "minmax(0, 1fr) var(--w-autonomy-cycle) var(--w-autonomy-by) var(--h-control-sm)",
};

/** The panel for a tab: its rows under a band that names the columns. A column stays where a row lacks its control, so the reset column holds its place down the list. */
export function AutonomyPanel({ take, tab, rows, onSet }: { take: AutonomyTake; tab: string; rows: readonly Row[]; onSet: (kind: AutonomyKind, level: Autonomy | undefined) => void }) {
  const name = useId();
  return (
    <div className="armada-wm-step armada-wm-auto" data-take={take}>
      <div className="armada-wm-step__band armada-wm-auto__row armada-wm-auto__band" style={{ gridTemplateColumns: COLUMNS[take] }} aria-hidden="true">
        <span />
        {take === "table" ? LEVELS.map((one) => <span key={one} className="armada-wm-auto__center">{SAID[one]}</span>) : <span />}
        <span>Set by</span>
        <span />
      </div>
      <ul className="armada-wm-auto__rows" aria-label="Autonomy">
        {rows.map((row) => {
          const group = `${name}-${tab}-${row.kind}`;
          const reset = `Use ${row.above.name}'s: ${SAID[row.above.value]}`;
          return (
            <li key={row.kind} className="armada-wm-auto__row" data-own={row.own ? "" : undefined} style={{ gridTemplateColumns: COLUMNS[take] }}>
              <Tooltip label={row.acts}>
                <span className="armada-wm-auto__kind">{row.said}</span>
              </Tooltip>
              {take === "table" ? (
                LEVELS.map((one) => (
                  <span key={one} className="armada-wm-auto__center armada-wm-auto__cell">
                    <Radio name={group} aria-label={`${row.said}: ${SAID[one]}`} checked={row.value === one} onChange={() => onSet(row.kind, one)}>
                      <span className="armada-wm-hidden">{SAID[one]}</span>
                    </Radio>
                  </span>
                ))
              ) : take === "segmented" ? (
                <span className="armada-wm-auto__segments" role="radiogroup" aria-label={row.said}>
                  {LEVELS.map((one) => (
                    <label key={one} className="armada-wm-auto__segment">
                      <input type="radio" name={group} className="armada-wm-auto__segment-input" checked={row.value === one} onChange={() => onSet(row.kind, one)} />
                      <span className="armada-wm-auto__segment-face">{SAID[one]}</span>
                    </label>
                  ))}
                </span>
              ) : (
                <span className="armada-wm-auto__cycle">
                  <Tooltip label={`Press for ${SAID[LEVELS[(LEVELS.indexOf(row.value) + 1) % LEVELS.length]!]}`}>
                    <Button variant="secondary" size="sm" ground="sunken" aria-label={`${row.said}: ${SAID[row.value]}`} onClick={() => onSet(row.kind, LEVELS[(LEVELS.indexOf(row.value) + 1) % LEVELS.length]!)}>
                      {SAID[row.value]}
                    </Button>
                  </Tooltip>
                </span>
              )}
              <span className="armada-wm-auto__by" data-own={row.own ? "" : undefined}>
                {row.setBy}
              </span>
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
