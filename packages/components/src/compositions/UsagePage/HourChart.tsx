import type { MouseEvent } from "react";

import { Skeleton } from "../../primitives/Skeleton/Skeleton";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { buildTip, startedAt } from "./usage-builds";
import { SHOWN, hourName } from "./usage-hours";
import type { Marker, UsageHours } from "./usage-hours";
import { tickStep } from "./usage-view";

/**
 * The last 48 hours as one or more line charts on a shared hour axis, a point an hour. Each plot has its own scale and its
 * own lines; an hour with nothing in it breaks the line, it never falls to zero. A build's marker runs the full height through
 * every plot: solid for a build on main, dashed for a preview. The picked build's span is shaded behind the lines. Pressing a
 * marker picks its build, as pressing its row in the Builds panel does.
 *
 * Keys are the page's: the chart is handed the hour its readout and crosshair stand on.
 */

const W = 520;
const LEFT = 36;
const RIGHT = 64;
const PAD = 12;
const GAP = 8;
const HEAD = 18;
const AXIS = 34;
const PLOT_W = W - LEFT - RIGHT;

export type HourSeries = {
  key: string;
  /** What the line is called in the readout and at its end. */
  name: string;
  /** The colour's own attribute: a model or a time bucket. */
  model?: string;
  time?: string;
  /** One value an hour, in the plot's unit; absent where the hour has none. */
  values: readonly (number | undefined)[];
};

export type HourPlot = {
  key: string;
  /** Drawn over the plot with its unit; absent where the panel's scope line names it. */
  title?: string;
  unit?: string;
  height: number;
  series: readonly HourSeries[];
  /** A value as the readout says it. */
  say: (value: number) => string;
  /** Name the lines at their ends. */
  direct?: boolean;
  /** Add a line to the readout with the lines' sum. */
  sum?: string;
  /** Grid lines at every step; otherwise at the foot and the top only. */
  steps?: boolean;
};

type Box = {
  top: number;
  bottom: number;
};

/** Where each plot sits, and how tall the whole drawing is. */
export function layout(plots: readonly Pick<HourPlot, "height" | "title">[]): { boxes: Box[]; height: number } {
  let y = PAD;
  const boxes = plots.map((one) => {
    const top = y + (one.title === undefined ? 0 : HEAD);
    const box = {
      top,
      bottom: top + one.height,
    };
    y = box.bottom + GAP;
    return box;
  });
  return {
    boxes,
    height: boxes[boxes.length - 1]!.bottom + AXIS,
  };
}

const x = (offset: number): number => LEFT + (offset / SHOWN) * PLOT_W;

const tidy = (value: number): string => `${+value.toFixed(1)}`;

/** The line's segments: runs of consecutive hours that hold a value. */
function runs(values: readonly (number | undefined)[]): number[][] {
  const found: number[][] = [];
  let run: number[] = [];
  values.forEach((one, at) => {
    if (one === undefined) {
      if (run.length > 0) found.push(run);
      run = [];
    } else run.push(at);
  });
  if (run.length > 0) found.push(run);
  return found;
}

/** Labels at the ends of the lines, kept a line's height apart. */
function spread(wanted: readonly number[], apart: number): number[] {
  const order = wanted.map((_, at) => at).sort((a, b) => wanted[a]! - wanted[b]!);
  const placed = [...wanted];
  let last = Number.NEGATIVE_INFINITY;
  for (const at of order) {
    placed[at] = Math.max(wanted[at]!, last + apart);
    last = placed[at]!;
  }
  return placed;
}

type Props = {
  hours: UsageHours;
  plots: readonly HourPlot[];
  label: string;
  markers: readonly Marker[];
  pick: string | undefined;
  onPick: (id: string) => void;
  /** The hour the crosshair and readout stand on. */
  hour: number | undefined;
  onHover: (hour: number | undefined) => void;
  onSelect: (hour: number) => void;
};

export function HourChart({ hours, plots, label, markers, pick, onPick, hour, onHover, onSelect }: Props) {
  const { boxes, height } = layout(plots);
  const first = boxes[0]!.top;
  const last = boxes[boxes.length - 1]!.bottom;
  const picked = markers.find((one) => one.build.id === pick);
  const at = (event: MouseEvent<HTMLDivElement>): number | undefined => {
    const box = event.currentTarget.getBoundingClientRect();
    const offset = ((((event.clientX - box.left) / box.width) * W - LEFT) / PLOT_W) * SHOWN;
    const found = Math.floor(offset);
    return offset >= 0 && found < SHOWN ? found : undefined;
  };
  const scales = plots.map((one, index) => {
    const peak = Math.max(0, ...one.series.flatMap((line) => line.values.filter((v): v is number => v !== undefined)));
    const step = tickStep(peak);
    const top = Math.max(step, Math.ceil(peak / step) * step);
    const box = boxes[index]!;
    return {
      top,
      step,
      y: (value: number) => box.bottom - ((box.bottom - box.top) * value) / top,
    };
  });
  const marks = Array.from(
    {
      length: SHOWN + 1,
    },
    (_, offset) => offset,
  ).filter((offset) => new Date(Date.parse(hours[0]!.at) + offset * 3_600_000).getHours() % 6 === 0);
  const hourAt = (offset: number): Date => new Date(Date.parse(hours[0]!.at) + offset * 3_600_000);
  const dayAt = (offset: number): string => {
    const day = hourAt(offset);
    return day.toLocaleString("en-US", {
      month: "short",
      day: "numeric",
    });
  };
  const readout = hour === undefined ? undefined : hours[hour];
  return (
    <div
      className="armada-usage__plot armada-hours"
      onMouseMove={(event) => onHover(at(event))}
      onMouseLeave={() => onHover(undefined)}
      onClick={(event) => {
        const found = at(event);
        if (found !== undefined) onSelect(found);
      }}
    >
      <svg
        className="armada-usage__chart"
        viewBox={`0 0 ${W} ${height}`}
        role="img"
        aria-label={label}
      >
        {plots.map((plot, index) => {
          const box = boxes[index]!;
          const scale = scales[index]!;
          const ticks = plot.steps
            ? Array.from(
                {
                  length: Math.round(scale.top / scale.step) + 1,
                },
                (_, i) => i * scale.step,
              )
            : [
                0,
                scale.top,
              ];
          return (
            <g key={plot.key}>
              {plot.title === undefined ? null : (
                <>
                  <text className="armada-usage__direct" x={LEFT} y={box.top - 6}>
                    {plot.title}
                  </text>
                  {plot.unit === undefined ? null : (
                    <text className="armada-usage__tick" x={W - RIGHT} y={box.top - 6} textAnchor="end">
                      {plot.unit}
                    </text>
                  )}
                </>
              )}
              {ticks.map((tick) => (
                <g key={tick}>
                  <line
                    className={tick === 0 ? "armada-usage__base" : "armada-usage__grid"}
                    x1={LEFT}
                    x2={W - RIGHT}
                    y1={scale.y(tick)}
                    y2={scale.y(tick)}
                  />
                  <text className="armada-usage__tick" x={LEFT - 6} y={scale.y(tick) + 4} textAnchor="end">
                    {tidy(tick)}
                  </text>
                </g>
              ))}
              {picked === undefined ? null : (
                <rect
                  className="armada-usage__daywash"
                  x={x(picked.at)}
                  y={box.top}
                  width={x(picked.to) - x(picked.at)}
                  height={box.bottom - box.top}
                />
              )}
            </g>
          );
        })}
        {plots.map((plot, index) => {
          const scale = scales[index]!;
          return (
            <g key={plot.key}>
              {markers.map((one) => (
                <line
                  key={one.build.id}
                  className="armada-hours__marker"
                  data-preview={one.build.branches.length > 0 || undefined}
                  data-picked={one.build.id === pick || undefined}
                  x1={x(one.at)}
                  x2={x(one.at)}
                  y1={boxes[index]!.top}
                  y2={boxes[index]!.bottom}
                />
              ))}
              {plot.series.map((line) => {
                const attr = {
                  "data-model": line.model,
                  "data-time": line.time,
                };
                return (
                  <g key={line.key}>
                    {runs(line.values).map((run) =>
                      run.length === 1 ? (
                        <circle
                          key={run[0]}
                          className="armada-hours__dot"
                          {...attr}
                          cx={x(run[0]! + 0.5)}
                          cy={scale.y(line.values[run[0]!]!)}
                          r={2}
                        />
                      ) : (
                        <polyline
                          key={run[0]}
                          className="armada-hours__line"
                          {...attr}
                          points={run.map((i) => `${x(i + 0.5)},${scale.y(line.values[i]!)}`).join(" ")}
                        />
                      ),
                    )}
                  </g>
                );
              })}
              {plot.direct !== true
                ? null
                : (() => {
                    const ends = plot.series.flatMap((line) => {
                      const end = line.values.reduce<number>((found, v, i) => (v === undefined ? found : i), -1);
                      return end < 0
                        ? []
                        : [
                            {
                              line,
                              end,
                              y: scale.y(line.values[end]!) + 4,
                            },
                          ];
                    });
                    const placed = spread(
                      ends.map((one) => one.y),
                      14,
                    );
                    return ends.map((one, i) => (
                      <text
                        key={one.line.key}
                        className="armada-usage__direct"
                        x={x(one.end + 0.5) + 8}
                        y={placed[i]}
                      >
                        {one.line.name}
                      </text>
                    ));
                  })()}
              {hour === undefined
                ? null
                : plot.series.map((line) => {
                    const value = line.values[hour];
                    return value === undefined ? null : (
                      <circle
                        key={line.key}
                        className="armada-hours__point"
                        data-model={line.model}
                        data-time={line.time}
                        cx={x(hour + 0.5)}
                        cy={scale.y(value)}
                        r={4}
                      />
                    );
                  })}
            </g>
          );
        })}
        {marks.map((offset) => (
          <g key={offset}>
            <text className="armada-usage__tick" x={x(offset)} y={last + 16} textAnchor="middle">
              {`${String(hourAt(offset).getHours()).padStart(2, "0")}h`}
            </text>
            {hourAt(offset).getHours() !== 0 ? null : (
              <text className="armada-usage__tick" data-on x={x(offset)} y={last + 30} textAnchor="middle">
                {dayAt(offset)}
              </text>
            )}
          </g>
        ))}
        {hour === undefined ? null : (
          <line
            className="armada-usage__cross"
            x1={x(hour + 0.5)}
            x2={x(hour + 0.5)}
            y1={first - (plots[0]!.title === undefined ? 0 : HEAD)}
            y2={last}
          />
        )}
      </svg>
      <div className="armada-usage__hits">
        {markers.map((one) => (
          <Tooltip
            key={one.build.id}
            asChild
            label={`${one.build.id} at ${startedAt(one.build.started)}. ${buildTip(one.build)}`}
          >
            <button
              type="button"
              tabIndex={-1}
              className="armada-hours__hit"
              aria-label={`${one.build.id}, ${startedAt(one.build.started)}${one.build.branches.length > 0 ? ", preview" : ", main"}`}
              aria-pressed={one.build.id === pick}
              style={{
                left: `${(x(one.at) / W) * 100}%`,
                top: `${(first / height) * 100}%`,
                height: `${((last - first) / height) * 100}%`,
              }}
              onClick={(event) => {
                event.stopPropagation();
                event.currentTarget.blur();
                onPick(one.build.id);
              }}
            />
          </Tooltip>
        ))}
      </div>
      {readout === undefined || hour === undefined ? null : (
        <div
          className="armada-usage__pop"
          role="status"
          data-flip={hour >= SHOWN / 2 || undefined}
          style={{
            left: `${(x(hour + 0.5) / W) * 100}%`,
          }}
        >
          <span className="armada-usage__popday">{hourName(hours, hour)}</span>
          {plots.flatMap((plot) => {
            const lines = plot.series.flatMap((line) => {
              const value = line.values[hour];
              return value === undefined
                ? []
                : [
                    {
                      line,
                      value,
                    },
                  ];
            });
            const sum = lines.reduce((total, one) => total + one.value, 0);
            return [
              ...[
                ...lines,
              ]
                .sort((a, b) => b.value - a.value)
                .map((one) => (
                  <span key={one.line.key} className="armada-usage__popline">
                    <span
                      className={one.line.model === undefined ? "armada-time__swatch" : "armada-usage__swatch"}
                      data-model={one.line.model}
                      data-time={one.line.time}
                      aria-hidden
                    />
                    <span>{one.line.name}</span>
                    <span className="armada-usage__fig">{plot.say(one.value)}</span>
                  </span>
                )),
              ...(plot.sum === undefined || lines.length === 0
                ? []
                : [
                    <span key="sum" className="armada-usage__popline" data-total>
                      <span />
                      <span>{plot.sum}</span>
                      <span className="armada-usage__fig">{plot.say(sum)}</span>
                    </span>,
                  ]),
            ];
          })}
        </div>
      )}
    </div>
  );
}

/** The chart's non-visual twin: every hour with what it holds, read by roles and not drawn. */
export function HourRows({
  hours,
  plots,
  label,
}: {
  hours: UsageHours;
  plots: readonly HourPlot[];
  label: string;
}) {
  return (
    <ul className="armada-hours__twin" aria-label={label}>
      {hours.map((one, at) => {
        const said = plots.flatMap((plot) =>
          plot.series.flatMap((line) => {
            const value = line.values[at];
            return value === undefined ? [] : [`${line.name} ${plot.say(value)}`];
          }),
        );
        return said.length === 0 ? null : <li key={one.at}>{`${hourName(hours, at)}, ${said.join(", ")}`}</li>;
      })}
    </ul>
  );
}

const SKEL = [
  0.5,
  0.6,
  0.45,
  0.7,
  0.8,
  0.55,
  0.4,
  0.6,
  0.75,
  0.65,
  0.5,
  0.7,
];

/** Each plot's line as a row of short bars on a base line, laid over the same box the chart's own drawing fills. */
export function PendingHours({ plots }: { plots: readonly Pick<HourPlot, "height" | "title">[] }) {
  const { boxes, height } = layout(plots);
  const step = PLOT_W / SKEL.length;
  return (
    <div className="armada-usage__plot" aria-hidden>
      <div
        className="armada-hours__skel"
        style={{
          aspectRatio: `${W} / ${height}`,
        }}
      >
        {boxes.map((box, index) => (
          <span
            key={index}
            className="armada-hours__skel-base"
            style={{
              left: `${(LEFT / W) * 100}%`,
              width: `${(PLOT_W / W) * 100}%`,
              top: `${(box.bottom / height) * 100}%`,
            }}
          />
        ))}
        {boxes.flatMap((box, index) =>
          SKEL.map((fill, at) => (
            <Skeleton
              key={`${index}-${at}`}
              className="armada-hours__skel-dash"
              style={{
                left: `${((LEFT + at * step + step / 6) / W) * 100}%`,
                width: `${(((step * 2) / 3) / W) * 100}%`,
                top: `${((box.bottom - (box.bottom - box.top) * fill) / height) * 100}%`,
              }}
            />
          )),
        )}
      </div>
    </div>
  );
}
