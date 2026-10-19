import { ArrowDown, ArrowUp, Minus, Power, PowerOff } from "lucide-react";

import { Kbd } from "../../primitives/Kbd/Kbd";
import { Stamp, StampSurface } from "../../primitives/Stamp/Stamp";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { ageOf } from "../../time-format";
import { M, shortId } from "./usage-view";
import type { RuleView } from "./usage-view";

/**
 * Fleet's usage rules and what each one achieved, in the Usage page's centre column. A rule tells a Job once when it
 * crosses the rule's limit; a trip keeps the Job, the time, the figure that crossed, and the Job's burn per hour in the
 * 60 minutes before and after the tell.
 *
 * Keys: `1` to `4` pick the rule at that place and list its trips, `Esc` clears the pick. With a Job picked on the page,
 * only that Job's trips are listed. A rule with no trips lists nothing. The picked rule draws what the Job is told under
 * its row, with the newest listed trip's figure.
 */

/** A move under this share of the burn reads as no change. */
const STILL = 0.05;

function Effect({ before, after }: { before: number; after: number | undefined }) {
  if (after === undefined) return <span className="armada-usage__delta" />;
  const share = before > 0 ? (after - before) / before : 0;
  const percent = `${Math.abs(Math.round(share * 100))}%`;
  const still = Math.abs(share) < STILL;
  const tip = still ? "No change in the burn per hour" : `${share < 0 ? "Down" : "Up"} ${percent} in the burn per hour`;
  return (
    <Tooltip label={tip}>
      <span className="armada-usage__delta" aria-label={tip}>
        {still ? <Minus size={12} aria-hidden /> : share < 0 ? <ArrowDown size={12} aria-hidden /> : <ArrowUp size={12} aria-hidden />}
        {still ? "" : percent}
      </span>
    </Tooltip>
  );
}

type Props = {
  rules: RuleView[];
  /** Index of the picked rule, or -1. */
  pick: number;
  onPick: (at: number) => void;
  titles: ReadonlyMap<string, string> | undefined;
  /** The picked Job's id: only its trips are listed. */
  job: string | undefined;
  /** The instant "ago" counts from. */
  now: number;
};

export function UsageRules({ rules, pick, onPick, titles, job, now }: Props) {
  const rule = rules[pick];
  const trips = rule === undefined ? [] : rule.trips.filter((one) => job === undefined || one.id === job).sort((a, b) => Date.parse(b.at) - Date.parse(a.at));
  const told = rule?.tell.replace("{figure}", trips[0]?.figure ?? rule.tell_figure);
  const top = Math.max(0, ...trips.flatMap((one) => [one.before, one.after ?? 0]));
  return (
    <section className="armada-usage__panel" aria-label="Usage rules">
      <div className="armada-usage__band">
        <span>Usage rules</span>
        <span className="armada-usage__keys">
          {rules.map((one, at) => (
            <Kbd key={one.id}>{String(at + 1)}</Kbd>
          ))}
          <Kbd>Esc</Kbd>
        </span>
      </div>
      <ul className="armada-usage__rules" role="listbox" aria-label="Rules">
        {rules.map((one, at) => (
          <li key={one.id} role="option" aria-selected={at === pick} aria-label={`${one.name}, ${one.on ? "on" : "off"}, ${one.limit_tip}`} className="armada-usage__rule" onClick={() => onPick(at)}>
            <Tooltip label={one.on ? "On" : "Off"}>
              <span className="armada-usage__state" data-on={one.on || undefined}>{one.on ? <Power size={14} aria-hidden /> : <PowerOff size={14} aria-hidden />}</span>
            </Tooltip>
            <span className="armada-usage__title">{one.name}</span>
            <Tooltip label={one.limit_tip}>
              <span className="armada-usage__name armada-usage__limit">{one.limit}</span>
            </Tooltip>
            {at !== pick || told === undefined ? null : (
              <div className="armada-usage__tellrow">
                <Tooltip label="Sent to the Job once">
                  <span className="armada-usage__tell">{told}</span>
                </Tooltip>
              </div>
            )}
          </li>
        ))}
      </ul>
      {rule === undefined || trips.length === 0 ? null : (
        <>
          <div className="armada-usage__scope">
            <span className="armada-usage__title">{rule.name}</span>
            <span className="armada-usage__unit">M units per hour</span>
          </div>
          <div className="armada-usage__tripheads" aria-hidden>
            <span />
            <span>When</span>
            <span>Figure</span>
            <span>Before</span>
            <span>After</span>
            <span />
          </div>
          <StampSurface>
            <ul className="armada-usage__trips" aria-label={`${rule.name}, trips`}>
              {trips.map((one) => {
                const title = titles?.get(one.id) ?? shortId(one.id);
                const fill = (units: number | undefined) => `${top > 0 && units !== undefined ? (units / top) * 100 : 0}%`;
                return (
                  <li key={`${one.id}-${one.at}`} className="armada-usage__trip" aria-label={`${title}, ${ageOf(Date.parse(one.at), now)}, ${one.figure}, ${M(one.before)} million before${one.after === undefined ? "" : `, ${M(one.after)} million after`}`}>
                    <span className="armada-usage__what">
                      <Tooltip label={title}>
                        <span className="armada-usage__title" data-id={titles?.has(one.id) !== true || undefined}>{title}</span>
                      </Tooltip>
                      <span className="armada-usage__track" aria-hidden>
                        <span className="armada-usage__fill" data-before style={{ width: fill(one.before) }} />
                      </span>
                      <span className="armada-usage__track" aria-hidden>
                        <span className="armada-usage__fill" style={{ width: fill(one.after) }} />
                      </span>
                    </span>
                    <Stamp first="age" at={one.at} now={now} className="armada-usage__fig" />
                    <Tooltip label={rule.figure_tip}>
                      <span className="armada-usage__fig">{one.figure}</span>
                    </Tooltip>
                    <Tooltip label="Million units per hour, the 60 minutes before the tell">
                      <span className="armada-usage__fig">{M(one.before)}</span>
                    </Tooltip>
                    <Tooltip label="Million units per hour, the 60 minutes after the tell">
                      <span className="armada-usage__fig">{one.after === undefined ? "" : M(one.after)}</span>
                    </Tooltip>
                    <Effect before={one.before} after={one.after} />
                  </li>
                );
              })}
            </ul>
          </StampSurface>
        </>
      )}
    </section>
  );
}
