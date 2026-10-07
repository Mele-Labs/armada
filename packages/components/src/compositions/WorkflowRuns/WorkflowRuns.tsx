import { Briefcase, Check, CircleDot, FolderGit2, Package, Trash2, X } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { Select } from "../../primitives/Select/Select";
import { Switch } from "../../primitives/Switch/Switch";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { Source } from "../WorkflowCreator/def";
import {
  EVERY,
  MOCK_COMMANDS,
  MOCK_SKILLS,
  PLACE_DIR,
  whenSaid,
  type FiredRun,
  type ResolvedRun,
  type Run,
  type RunKind,
  type RunWhen,
} from "./runs";

/**
 * Runs: a script or a skill that fires at a moment in a Job. The rows are the
 * list the Workflow creator draws twice, beside the workflows and on a step,
 * and each opens the same fields; `FiredRuns` is what Job detail draws once
 * they have fired. Mock only: no Fleet behind it.
 */

/** What each place is, said once on its mark. */
const PLACE: Record<Source, { Glyph: LucideIcon; said: string }> = {
  carried: { Glyph: Package, said: "Armada default" },
  repository: { Glyph: FolderGit2, said: "This repository" },
  kit: { Glyph: Briefcase, said: "This machine" },
};

function PlaceMark({ place }: { place: Source }) {
  const { Glyph, said } = PLACE[place];
  return (
    <Tooltip label={said}>
      <span className="armada-runs__mark" role="img" aria-label={said}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** The runs as rows, each marked by where it is set. A shadowed run is drawn quiet. */
export function RunRows({ runs, label, onOpen }: { runs: readonly ResolvedRun[]; label: string; onOpen: (id: string) => void }) {
  return (
    <ul className="armada-runs" aria-label={label}>
      {runs.map((one) => {
        const over = one.overriddenBy === undefined ? undefined : `Overridden by ${PLACE[one.overriddenBy].said.toLowerCase()}`;
        return (
          <li key={one.id} className="armada-runs__row" data-state={over === undefined ? undefined : "overridden"}>
            <button
              type="button"
              aria-label={`${one.name}, ${whenSaid(one)}, ${PLACE[one.place].said.toLowerCase()}${over === undefined ? "" : ", overridden"}`}
              onClick={() => onOpen(one.id)}
            >
              <PlaceMark place={one.place} />
              <span className="armada-runs__name">{one.name}</span>
              <span className="armada-runs__when">{whenSaid(one)}</span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}

/** The fields of one run, in the app's overlay panel. */
export function RunFields({
  run,
  workflow,
  steps,
  onChange,
  onRemove,
}: {
  run: Run;
  /** The workflow open beside the panel, which "This workflow" means. Absent where none is. */
  workflow: string | undefined;
  /** That workflow's steps. */
  steps: readonly string[];
  onChange: (next: Partial<Run>) => void;
  onRemove: () => void;
}) {
  const names = run.kind === "command" ? MOCK_COMMANDS : MOCK_SKILLS;
  const held = names.includes(run.name as never) ? names : [...names, run.name];
  const applies = run.applies === EVERY || run.applies === workflow ? [] : [run.applies];
  const stepped = run.when !== "pr_opened";
  return (
    <>
      <Select label="When" value={run.when} onChange={(event) => onChange({ when: event.target.value as RunWhen })}>
        <option value="starts">A step starts</option>
        <option value="passes">A step passes</option>
        <option value="pr_opened">PR opened</option>
      </Select>
      {stepped ? (
        run.applies !== EVERY && workflow === run.applies ? (
          <Select label="Step" value={run.step} onChange={(event) => onChange({ step: event.target.value })}>
            <option value="" />
            {steps.map((one) => (
              <option key={one} value={one}>
                {one}
              </option>
            ))}
          </Select>
        ) : (
          <Input label="Step" mono value={run.step} onChange={(event) => onChange({ step: event.target.value })} />
        )
      ) : null}
      <Select label="Applies to" value={run.applies} onChange={(event) => onChange({ applies: event.target.value })}>
        <option value={EVERY}>Every workflow</option>
        {workflow === undefined ? null : <option value={workflow}>{`This workflow, ${workflow}`}</option>}
        {applies.map((one) => (
          <option key={one} value={one}>{`Workflow ${one}`}</option>
        ))}
      </Select>
      <Select
        label="Set in"
        value={run.place}
        onChange={(event) => onChange({ place: event.target.value as Source })}
      >
        {run.place === "carried" ? <option value="carried">Armada default</option> : null}
        <option value="repository">Repository</option>
        <option value="kit">This machine</option>
      </Select>
      <span className="armada-runs__fact">{`${PLACE_DIR[run.place]}/runs.toml`}</span>
      <Select
        label="Runs"
        value={run.kind}
        onChange={(event) => {
          const kind = event.target.value as RunKind;
          onChange({ kind, name: (kind === "command" ? MOCK_COMMANDS : MOCK_SKILLS)[0] });
        }}
      >
        <option value="command">Command in armada.yml</option>
        <option value="skill">Skill</option>
      </Select>
      <Select label={run.kind === "command" ? "Command" : "Skill"} value={run.name} onChange={(event) => onChange({ name: event.target.value })}>
        {held.map((one) => (
          <option key={one} value={one}>
            {one}
          </option>
        ))}
      </Select>
      <div className="armada-runs__fails" role="group" aria-label="If it fails">
        <span className="armada-runs__eyebrow">If it fails</span>
        <Switch checked={run.block} onChange={(event) => onChange({ block: event.target.checked })}>
          Block the Job
        </Switch>
        <Switch checked={run.repair} onChange={(event) => onChange({ repair: event.target.checked })}>
          Self repair
        </Switch>
      </div>
      <div>
        <Button variant="ghost" onClick={onRemove}>
          <Trash2 size={12} strokeWidth={2} aria-hidden /> Remove run
        </Button>
      </div>
    </>
  );
}

/** What Job detail draws for runs that fired: the result, or a repair Drone at work, and where each is set. */
export function FiredRuns({ runs }: { runs: readonly FiredRun[] }) {
  return (
    <ul className="armada-runs" aria-label="Runs that fired">
      {runs.map((one) => (
        <li key={`${one.name}|${one.when}`} className="armada-runs__row armada-runs__row--fired">
          <span className="armada-runs__fired">
            {one.state === "passed" ? (
              <Tooltip label="Passed">
                <span className="armada-runs__mark" data-hue="passed" role="img" aria-label="Passed">
                  <Check size={12} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
            ) : (
              <>
                <Tooltip label="Failed">
                  <span className="armada-runs__mark" data-hue="failed" role="img" aria-label="Failed">
                    <X size={12} strokeWidth={2} aria-hidden />
                  </span>
                </Tooltip>
                <Tooltip label="Repair Drone working">
                  <span className="armada-runs__mark" data-hue="running" data-pulsing="" role="img" aria-label="Repair Drone working">
                    <CircleDot size={12} strokeWidth={2} aria-hidden />
                  </span>
                </Tooltip>
              </>
            )}
          </span>
          <span className="armada-runs__name">{one.name}</span>
          <span className="armada-runs__when">{one.when}</span>
          <PlaceMark place={one.place} />
        </li>
      ))}
    </ul>
  );
}
