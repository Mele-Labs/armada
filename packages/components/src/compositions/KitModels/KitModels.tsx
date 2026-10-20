import { useKept } from "../../keep";
import { Input } from "../../primitives/Input/Input";
import { Select } from "../../primitives/Select/Select";
import { Switch } from "../../primitives/Switch/Switch";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { holds } from "../KitBrowser/kit-rows";

/**
 * One model the agent offers, as Fleet's `list_models` answers it. Declared here until the protocol
 * carries it: the shape is the mock's, and the Fleet side is being built to roughly match.
 */
export type KitModel = {
  /** The family, lower case: `opus`. */
  name: string;
  /** The family as a person reads it, without a version: `Opus`. */
  display: string;
  /** The version id a Job runs: `opus-5-5`. */
  resolved: string;
  says: string;
  effort_levels: string[];
  /** Whether Jobs may use it. */
  on: boolean;
};

/** The model each kind of run starts on, and the effort each is given. */
export type KitModelChoices = {
  jobs: string;
  drones: string;
  reviews: string;
  effort: string;
  judge: string;
  judgeEffort: string;
  proposer: string;
  proposerEffort: string;
  retro: string;
  secondOpinion: string;
  downshift: string;
  helmEffort: string;
};
export type KitModelSlot = keyof KitModelChoices;

/** Fleet's first effort: passing none, the harness picking. */
const HARNESS_DEFAULT = "harness default";

export type KitModelsRead = { read: "read"; models: KitModel[]; choices: KitModelChoices } | { read: "not_read"; why: string };

/** A model's version as a fact: `opus-5-5` is `5.5`. */
export const versionOf = (resolved: string): string => /((?:-\d+)+)$/.exec(resolved)?.[1]?.slice(1).replace(/-/g, ".") ?? "";

/** The selects in reading order, two to a row: a model, then its effort where it has one. */
const SLOTS: readonly { slot: KitModelSlot; label: string; name?: string; effort?: true }[] = [
  { slot: "jobs", label: "Jobs" },
  { slot: "drones", label: "Drones" },
  { slot: "reviews", label: "Reviews" },
  { slot: "effort", label: "Effort", effort: true },
  { slot: "judge", label: "Judge" },
  { slot: "judgeEffort", label: "Effort", name: "Judge effort", effort: true },
  { slot: "proposer", label: "Proposer" },
  { slot: "proposerEffort", label: "Effort", name: "Proposer effort", effort: true },
  { slot: "retro", label: "Retro" },
  { slot: "secondOpinion", label: "Second opinion" },
  { slot: "downshift", label: "Step down to" },
  { slot: "helmEffort", label: "Helm effort", effort: true },
];

export type KitModelsProps = {
  /** `undefined` before the read lands. */
  models: KitModelsRead | undefined;
  keep: string;
  picked: string | undefined;
  onPick: (name: string) => void;
  onOn: (name: string, on: boolean) => void;
  onChoose: (slot: KitModelSlot, value: string) => void;
};

export function KitModels({ models, keep, picked, onPick, onOn, onChoose }: KitModelsProps) {
  const [filter, setFilter] = useKept(`${keep}:models:filter`, "");
  const read = models?.read === "read" ? models : undefined;
  const rows = (read?.models ?? []).filter((one) => holds(one.display, filter));
  const row = rows.find((one) => one.name === picked) ?? rows[0];
  const on = (read?.models ?? []).filter((one) => one.on);
  // The efforts the models take, led by Fleet's own first, and any in force.
  const efforts = [
    ...new Set([
      HARNESS_DEFAULT,
      ...on.flatMap((one) => one.effort_levels),
      ...(read === undefined ? [] : SLOTS.filter((one) => one.effort).map((one) => read.choices[one.slot])),
    ]),
  ];

  return (
    <div className="armada-kit-browser">
      <section className="armada-kit-browser__list" aria-label="Models" data-col="list">
        {read === undefined ? null : (
          <div className="armada-kit-models__choose">
            {SLOTS.map(({ slot, label, name, effort }) => (
              <Select key={slot} label={label} aria-label={name} value={read.choices[slot]} onChange={(event) => onChoose(slot, event.target.value)}>
                {effort
                  ? efforts.map((one) => (
                      <option key={one} value={one}>
                        {one}
                      </option>
                    ))
                  : on.map((one) => (
                      <option key={one.name} value={one.name}>
                        {one.display}
                      </option>
                    ))}
              </Select>
            ))}
          </div>
        )}
        <Input
          type="search"
          aria-label="Filter Models"
          placeholder="Filter"
          value={filter}
          onChange={(event) => setFilter(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Escape" && filter !== "") {
              event.preventDefault();
              event.stopPropagation();
              setFilter("");
            }
          }}
        />
        {models?.read === "not_read" ? (
          <p className="armada-kit-browser__not-read">
            <span className="armada-kit-browser__not-read-head">Not read</span>
            <span>{models.why}</span>
          </p>
        ) : (
          <div className="armada-scroll-area armada-kit-browser__scroll">
            <ul className="armada-kit-browser__items">
              {rows.map((one) => (
                <li key={one.name} className="armada-kit-browser__item">
                  <button
                    type="button"
                    className="armada-kit-browser__row"
                    data-nav-item=""
                    aria-current={one.name === row?.name ? "true" : undefined}
                    tabIndex={one.name === row?.name ? 0 : -1}
                    onClick={() => onPick(one.name)}
                  >
                    <span className="armada-kit-browser__name">{one.display}</span>
                    <span className="armada-kit-browser__side">
                      <Tooltip beside label="Version">
                        <span className="armada-kit-browser__meta">{versionOf(one.resolved)}</span>
                      </Tooltip>
                    </span>
                  </button>
                  <Tooltip beside label="Jobs may use it">
                    <span className="armada-kit-models__switch">
                      <Switch aria-label={`${one.display} may be used`} checked={one.on} disabled={one.on && on.length === 1} onChange={(event) => onOn(one.name, event.target.checked)}>
                        {""}
                      </Switch>
                    </span>
                  </Tooltip>
                </li>
              ))}
            </ul>
          </div>
        )}
      </section>

      <section className="armada-kit-browser__detail" aria-label="Detail" data-col="detail">
        {row === undefined ? null : (
          <article className="armada-kit-browser__item-detail" aria-label={row.display}>
            <header className="armada-kit-browser__head">
              <h3 className="armada-kit-browser__title">{row.display}</h3>
              <Tooltip beside label="Version">
                <span className="armada-kit-browser__meta">{versionOf(row.resolved)}</span>
              </Tooltip>
            </header>
            <p className="armada-kit-browser__path">{row.resolved}</p>
            <p className="armada-kit-browser__says">{row.says}</p>
            <ul className="armada-kit-models__efforts" aria-label="Effort levels">
              {row.effort_levels.map((level) => (
                <li key={level} className="armada-kit-browser__meta">
                  {level}
                </li>
              ))}
            </ul>
          </article>
        )}
      </section>
    </div>
  );
}
