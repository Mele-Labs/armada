// One Job or Session as a tile: the state icon at the top left naming what is active, the kind as a
// small glyph at the right, the title wrapping to two lines, and a bottom strip of the Job's step
// pips or a Session's activity, with the age. The state is in the frame: a tile that needs the owner
// is lit. A Session never draws pips, and a Job draws them only from two steps. Props are plain so
// any grid can lay tiles out.

import { useEffect, useRef, type KeyboardEvent } from "react";
import { LoaderCircle, SquareTerminal, Workflow } from "lucide-react";
import { Tooltip } from "@armada/components";

import { age, type Item } from "./Dashboard";

export function FleetTile({ item, selected, onSelect, now = Date.now() }: { item: Item; selected: boolean; onSelect: (key: string) => void; now?: number }) {
  const ref = useRef<HTMLLIElement>(null);
  useEffect(() => {
    if (selected) ref.current?.scrollIntoView({ block: "nearest" });
  }, [selected]);
  const spins = item.mark === undefined && item.live === true;
  const State = item.mark ?? (spins ? LoaderCircle : item.icon);
  const said = item.state ?? item.kind;
  const lit = item.hue === "ask" || item.hue === "issue";
  const kind = item.job !== undefined ? { Glyph: Workflow, said: "Job" } : item.session === true ? { Glyph: SquareTerminal, said: "Session" } : undefined;
  const steps = item.steps !== undefined && item.steps.length >= 2 ? item.steps : undefined;
  return (
    <li
      ref={ref}
      role="option"
      className="armada-tile"
      data-hue={item.hue}
      data-lit={lit || undefined}
      data-job-id={item.job?.id}
      data-status={item.job?.status}
      aria-label={`${item.title}, ${kind?.said ?? item.kind}`}
      aria-selected={selected}
      onClick={() => onSelect(item.key)}
    >
      <div className="armada-tile__head">
        <Tooltip label={said}>
          <span className="armada-tile__mark" role="img" aria-label={said} data-spin={spins || undefined} data-live={(!spins && (item.live === true || lit)) || undefined}>
            <State size={20} aria-hidden="true" />
          </span>
        </Tooltip>
        {kind === undefined ? null : (
          <Tooltip label={kind.said}>
            <span className="armada-tile__kind">
              <kind.Glyph size={14} aria-hidden="true" />
            </span>
          </Tooltip>
        )}
      </div>
      <span className="armada-tile__title">{item.title}</span>
      <div className="armada-tile__foot">
        {steps !== undefined ? (
          <ol className="armada-tile__pips" aria-label="Steps">
            {steps.map((step, index) => (
              <li key={step.id} data-pip={index < (item.stepAt ?? 0) ? "done" : index === item.stepAt ? "live" : "ahead"}>
                <Tooltip label={step.label}>
                  <span className="armada-tile__pip" role="img" aria-label={step.label} />
                </Tooltip>
              </li>
            ))}
          </ol>
        ) : item.spark !== undefined && item.spark.length > 0 ? (
          <Tooltip label="Recent activity">
            <span className="armada-tile__spark" role="img" aria-label="Recent activity">
              {item.spark.map((height, index) => (
                <span key={index} style={{ ["--h" as string]: height }} />
              ))}
            </span>
          </Tooltip>
        ) : (
          <span />
        )}
        {item.at === undefined ? null : (
          <Tooltip label="Age">
            <span className="armada-tile__age">{age(item.at, now)}</span>
          </Tooltip>
        )}
      </div>
    </li>
  );
}

/** The arrows over a grid of tiles: left and right step by one, up and down by a row as drawn. */
export function onTilesKey(event: KeyboardEvent, rows: readonly { key: string }[], at: number, pick: (key: string) => void): void {
  const columns = Math.max(1, getComputedStyle(event.currentTarget).gridTemplateColumns.split(" ").length);
  const step = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : event.key === "ArrowDown" ? columns : event.key === "ArrowUp" ? -columns : 0;
  if (step === 0) return;
  event.preventDefault();
  const next = rows[Math.min(rows.length - 1, Math.max(0, at + step))];
  if (next !== undefined) pick(next.key);
}
