import { ArrowDown, ArrowUp } from "lucide-react";
import { Fragment, useLayoutEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import {
  CartesianGrid,
  Line,
  LineChart,
  ReferenceArea,
  ReferenceDot,
  ReferenceLine,
  ResponsiveContainer,
  XAxis,
  YAxis,
  usePlotArea,
  useXAxisScale,
  useYAxisScale,
} from "recharts";
import type { MouseHandlerDataParam } from "recharts/types/synchronisation/types";

import { Skeleton } from "../../primitives/Skeleton/Skeleton";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { FIGURE_NAME, buildTip, changeOf, commitOf, shown, startedAt } from "./usage-builds";
import type { Build, Figure } from "./usage-builds";
import { SHOWN, hourName, markersOf } from "./usage-hours";
import type { Marker, UsageHours } from "./usage-hours";
import { tickStep } from "./usage-view";

/**
 * The last 48 hours as one or more line charts on a shared hour axis, a point an hour, drawn with Recharts. Each plot has its
 * own scale and its own lines; an hour with nothing in it breaks the line, it never falls to zero. A build is a line through
 * every plot, solid for a build on main and dashed for a preview, with a flag on a rail above the plot that moved most: the
 * flag keeps its commit and change where the build moved a figure by the page's STILL or more. The picked build's span is
 * shaded behind the lines. Pressing a flag or its line picks the build, as pressing its row in the Builds panel does.
 *
 * The hour under the pointer, or under the page's keys, is the page's state and is handed in: the crosshair, the dots and the
 * tooltip all stand on it, so the keys and the pointer drive one thing.
 */

/** The y axis's width, and the right margin: wide enough to name the lines at their ends, or just clear of the last tick. */
const LEFT = 36;
const NAMED = 64;
const EDGE = 12;
/** The hour axis's height, on the last plot. */
const AXIS = 34;
/** What a flag's width is reckoned at, for keeping flags clear of each other: a character, its padding and the change. */
const CHAR = 7;
const PAD = 86;
const SMALL = 20;
const LANES = 3;
/** The room a plot keeps above its top line and, but for the last, below its foot. */
const CROWN = 8;
const FOOT = 6;

const px = (n: number): string => `${n}px`;

export type HourSeries = {
  key: string;
  /** What the line is called in the tooltip and at its end. */
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
  /** The least the plot's height is, and the share of any spare height it takes. */
  height: number;
  /** The figure of a build this plot draws: a build that moved it keeps its change on its flag here. */
  figure: Figure;
  series: readonly HourSeries[];
  /** A value as the tooltip says it. */
  say: (value: number) => string;
  /** Name the lines at their ends. */
  direct?: boolean;
  /** Add a line to the tooltip with the lines' sum. */
  sum?: string;
  /** Grid lines at every step; otherwise at the foot and the top only. */
  steps?: boolean;
};

type Move = ReturnType<typeof changeOf>;

const hourAt = (hours: UsageHours, offset: number): Date =>
  new Date(Date.parse(hours[0]!.at) + offset * 3_600_000);

const tidy = (value: number): string => `${+value.toFixed(1)}`;

/** Labels at the ends of the lines, kept a line's height apart. */
function spread(wanted: readonly number[], apart: number): number[] {
  const order = wanted.map((_, at) => at).sort((a, b) => wanted[a]! - wanted[b]!);
  const placed = [
    ...wanted,
  ];
  let last = Number.NEGATIVE_INFINITY;
  for (const at of order) {
    placed[at] = Math.max(wanted[at]!, last + apart);
    last = placed[at]!;
  }
  return placed;
}

/** The width of the element, kept as it resizes. */
function useWidth() {
  const ref = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  useLayoutEffect(() => {
    const el = ref.current;
    if (el === null) return;
    const read = () => setWidth(el.getBoundingClientRect().width);
    read();
    if (typeof ResizeObserver === "undefined") return;
    const watch = new ResizeObserver(read);
    watch.observe(el);
    return () => watch.disconnect();
  }, []);
  return [
    ref,
    width,
  ] as const;
}

/** A change as the page's delta chip. */
function Delta({ move }: { move: NonNullable<Move> }) {
  return (
    <span className="armada-usage__delta" data-way={move.up ? "worse" : "better"}>
      {move.up ? <ArrowUp size={12} aria-hidden /> : <ArrowDown size={12} aria-hidden />}
      {`${Math.round(move.percent)}%`}
    </span>
  );
}

type Flag = {
  marker: Marker;
  plot: number;
  wide: boolean;
  lane: number;
  flip: boolean;
  /** The change on the plot's own figure, where it is a move. */
  move: NonNullable<Move> | undefined;
};

/**
 * Where each build's flag sits: on the plot whose figure it moved most, or the first. A flag that moved a figure, or is
 * picked, shows its commit; the rest are small. Flags on one rail take the first lane clear of the last in it, and one that
 * will not fit in the lanes there are shrinks.
 */
function flagsOf(
  markers: readonly Marker[],
  builds: readonly Build[],
  plots: readonly HourPlot[],
  pick: string | undefined,
  plotW: number,
): Flag[] {
  const ends = plots.map<number[][]>(() =>
    Array.from(
      {
        length: LANES,
      },
      () => [],
    ),
  );
  return markers.map((marker) => {
    const before = builds[builds.findIndex((one) => one.id === marker.build.id) + 1];
    const moves = plots.map((plot) => {
      const move = changeOf(marker.build, before, plot.figure);
      return move === undefined || move.still ? undefined : move;
    });
    const most = moves.reduce<number>(
      (found, move, at) =>
        move !== undefined && move.percent > (moves[found]?.percent ?? 0) ? at : found,
      0,
    );
    const move = moves[most];
    const picked = marker.build.id === pick;
    const where = ends[most]!;
    const x = (marker.at / SHOWN) * plotW;
    const place = (width: number) => {
      const flip = x + width > plotW + EDGE;
      const from = flip ? x - width : x;
      const lane = where.findIndex((used) => fits(used, from, from + width));
      return {
        flip,
        from,
        lane,
      };
    };
    let wide = move !== undefined || picked;
    let put = place(wide ? commitOf(marker.build).length * CHAR + PAD : SMALL);
    if (put.lane < 0 && wide && !picked) {
      wide = false;
      put = place(SMALL);
    }
    const lane = put.lane < 0 ? LANES - 1 : put.lane;
    const width = wide ? commitOf(marker.build).length * CHAR + PAD : SMALL;
    where[lane]!.push(put.from, put.from + width);
    return {
      marker,
      plot: most,
      wide,
      lane,
      flip: put.flip,
      move,
    };
  });
}

/** Whether a span clears every span already in a lane, kept as from, to pairs. */
function fits(used: readonly number[], from: number, to: number): boolean {
  for (let at = 0; at < used.length; at += 2)
    if (from < used[at + 1]! + 4 && to > used[at]! - 4) return false;
  return true;
}

/** The hour axis's tick: its hour, and under midnight the day. */
function HourTick({
  x,
  y,
  payload,
  hours,
}: {
  x?: number;
  y?: number;
  payload?: {
    value: number;
  };
  hours: UsageHours;
}) {
  if (x === undefined || y === undefined || payload === undefined) return <g />;
  const at = hourAt(hours, payload.value);
  return (
    <g>
      <text className="armada-hours__tick" x={x} y={y + 12} textAnchor="middle">
        {`${String(at.getHours()).padStart(2, "0")}h`}
      </text>
      {at.getHours() !== 0 ? null : (
        <text className="armada-hours__tick" data-on x={x} y={y + 26} textAnchor="middle">
          {at.toLocaleString("en-US", {
            month: "short",
            day: "numeric",
          })}
        </text>
      )}
    </g>
  );
}

/** Each line's name at its last point, kept apart. */
function EndNames({ series }: { series: readonly HourSeries[] }) {
  const across = useXAxisScale();
  const up = useYAxisScale();
  const area = usePlotArea();
  if (across === undefined || up === undefined || area === undefined) return null;
  const ends = series.flatMap((line) => {
    const end = line.values.reduce<number>((found, v, i) => (v === undefined ? found : i), -1);
    const at = up(line.values[end]);
    return end < 0 || at === undefined
      ? []
      : [
          {
            line,
            end,
            y: at + 4,
          },
        ];
  });
  const placed = spread(
    ends.map((one) => one.y),
    14,
  );
  return (
    <g>
      {ends.map((one, i) => (
        <text
          key={one.line.key}
          className="armada-hours__direct"
          x={(across(one.end + 0.5) ?? area.x + area.width) + 8}
          y={placed[i]}
        >
          {one.line.name}
        </text>
      ))}
    </g>
  );
}

type Props = {
  hours: UsageHours;
  plots: readonly HourPlot[];
  label: string;
  /** Every build, newest first: each marker reads against the one before it. */
  builds: readonly Build[];
  pick: string | undefined;
  onPick: (id: string) => void;
  /** The hour the crosshair and tooltip stand on. */
  hour: number | undefined;
  onHover: (hour: number | undefined) => void;
  onSelect: (hour: number) => void;
};

/** The chart's own measures, handed to the stylesheet so its overlays sit on the plot Recharts draws. */
const frame = (named: boolean): CSSProperties =>
  ({
    "--plot-left": px(LEFT),
    "--plot-right": px(named ? NAMED : EDGE),
    "--axis": px(AXIS),
  }) as CSSProperties;

const along = (offset: number): string =>
  `calc(var(--plot-left) + (100% - var(--plot-left) - var(--plot-right)) * ${offset / SHOWN})`;

export function HourChart({
  hours,
  plots,
  label,
  builds,
  pick,
  onPick,
  hour,
  onHover,
  onSelect,
}: Props) {
  const [ref, width] = useWidth();
  const named = plots.some((one) => one.direct === true);
  const right = named ? NAMED : EDGE;
  const markers = markersOf(hours, builds);
  const picked = markers.find((one) => one.build.id === pick);
  const flags = flagsOf(markers, builds, plots, pick, Math.max(0, width - LEFT - right));
  const running =
    hour === undefined ? undefined : markers.filter((one) => one.at <= hour + 0.5).at(-1);
  const figures = [
    ...new Set(plots.map((one) => one.figure)),
  ];
  const lines = (foot: number) => (
    <div
      className="armada-hours__lines"
      aria-hidden
      style={
        {
          "--foot": px(foot),
        } as CSSProperties
      }
    >
      {markers.map((one) => (
        <div
          key={one.build.id}
          className="armada-hours__marker"
          data-preview={one.build.branches.length > 0 || undefined}
          data-picked={one.build.id === pick || undefined}
          style={{
            left: `${(one.at / SHOWN) * 100}%`,
          }}
          onClick={() => onPick(one.build.id)}
        />
      ))}
    </div>
  );
  const hereAt = (state: MouseHandlerDataParam): number | undefined => {
    const found = Number(state.activeTooltipIndex);
    return state.activeTooltipIndex == null || !Number.isInteger(found) ? undefined : found;
  };
  const ticks = Array.from(
    {
      length: SHOWN + 1,
    },
    (_, offset) => offset,
  ).filter((offset) => hourAt(hours, offset).getHours() % 6 === 0);

  return (
    <div
      className="armada-usage__plot armada-hours"
      role="group"
      aria-label={label}
      style={frame(named)}
    >
      <div ref={ref} className="armada-hours__stack">
        {plots.map((plot, index) => {
          const last = index === plots.length - 1;
          const peak = Math.max(
            0,
            ...plot.series.flatMap((line) =>
              line.values.filter((v): v is number => v !== undefined),
            ),
          );
          const step = tickStep(peak);
          const top = Math.max(step, Math.ceil(peak / step) * step);
          const marks = plot.steps
            ? Array.from(
                {
                  length: Math.round(top / step) + 1,
                },
                (_, i) => i * step,
              )
            : [
                0,
                top,
              ];
          const data = hours.map((_, at) => ({
            x: at + 0.5,
            ...Object.fromEntries(
              plot.series.map((line) => [
                line.key,
                line.values[at],
              ]),
            ),
          }));
          const mine = flags.filter((one) => one.plot === index);
          const lanes = mine.reduce((most, one) => Math.max(most, one.lane + 1), 0);
          return (
            <div
              key={plot.key}
              className="armada-hours__plot"
              style={{
                flexGrow: plot.height,
              }}
            >
              {plot.title === undefined ? null : (
                <div className="armada-hours__head">
                  <span className="armada-usage__title">{plot.title}</span>
                  {plot.unit === undefined ? null : (
                    <span className="armada-usage__unit">{plot.unit}</span>
                  )}
                </div>
              )}
              {lanes === 0 ? null : (
                <div
                  className="armada-hours__rail"
                  style={
                    {
                      "--lanes": lanes,
                    } as CSSProperties
                  }
                >
                  {mine.map((one) => {
                    const build = one.marker.build;
                    const preview = build.branches.length > 0;
                    const before = builds[builds.findIndex((b) => b.id === build.id) + 1];
                    const said = figures.map((figure) => {
                      const move = changeOf(build, before, figure);
                      return move === undefined || move.still
                        ? ""
                        : `, ${FIGURE_NAME[figure]} ${move.up ? "up" : "down"} ${Math.round(move.percent)} percent`;
                    });
                    return (
                      <Fragment key={build.id}>
                        <span
                          className="armada-hours__pole"
                          data-preview={preview || undefined}
                          data-picked={build.id === pick || undefined}
                          style={{
                            left: along(one.marker.at),
                            top: `calc(var(--flag) * ${one.lane})`,
                          }}
                          aria-hidden
                        />
                        <span
                          className="armada-hours__flagpos"
                          data-flip={one.flip || undefined}
                          style={{
                            left: along(one.marker.at),
                            top: `calc(var(--flag) * ${one.lane})`,
                          }}
                        >
                          <Tooltip
                            asChild
                            label={
                              <span className="armada-hours__card">
                                <span className="armada-hours__cardhead">
                                  <span>{commitOf(build)}</span>
                                  <span>{startedAt(build.started)}</span>
                                </span>
                                <span>{buildTip(build)}</span>
                                {figures.map((figure) => {
                                  const move = changeOf(build, before, figure);
                                  return (
                                    <span key={figure} className="armada-hours__cardrow">
                                      <span>{FIGURE_NAME[figure]}</span>
                                      <span className="armada-usage__fig">
                                        {shown(build, figure)}
                                      </span>
                                      <span>
                                        {move === undefined || move.still ? null : (
                                          <Delta move={move} />
                                        )}
                                      </span>
                                    </span>
                                  );
                                })}
                              </span>
                            }
                          >
                            <button
                              type="button"
                              tabIndex={-1}
                              className="armada-hours__flag"
                              data-preview={preview || undefined}
                              data-picked={build.id === pick || undefined}
                              data-wide={one.wide || undefined}
                              aria-label={`${commitOf(build)}, ${startedAt(build.started)}${preview ? ", preview" : ", main"}${said.join("")}`}
                              aria-pressed={build.id === pick}
                              onClick={(event) => {
                                event.currentTarget.blur();
                                onPick(build.id);
                              }}
                            >
                              {one.wide ? (
                                <span className="armada-hours__commit">{commitOf(build)}</span>
                              ) : null}
                              {one.move === undefined ? null : <Delta move={one.move} />}
                            </button>
                          </Tooltip>
                        </span>
                      </Fragment>
                    );
                  })}
                </div>
              )}
              <div
                className="armada-hours__canvas"
                style={{
                  minHeight: plot.height + (last ? AXIS : 0),
                }}
              >
                <ResponsiveContainer width="100%" height="100%">
                  <LineChart
                    data={data}
                    margin={{
                      top: CROWN,
                      right,
                      bottom: last ? 0 : FOOT,
                      left: 0,
                    }}
                    accessibilityLayer={false}
                    onMouseMove={(state) => onHover(hereAt(state))}
                    onMouseLeave={() => onHover(undefined)}
                    onClick={(state) => {
                      const at = hereAt(state);
                      if (at !== undefined) onSelect(at);
                    }}
                  >
                    <CartesianGrid className="armada-hours__grid" vertical={false} />
                    <XAxis
                      dataKey="x"
                      type="number"
                      domain={[
                        0,
                        SHOWN,
                      ]}
                      ticks={ticks}
                      interval={0}
                      hide={!last}
                      height={AXIS}
                      axisLine={false}
                      tickLine={false}
                      tick={<HourTick hours={hours} />}
                    />
                    <YAxis
                      type="number"
                      domain={[
                        0,
                        top,
                      ]}
                      ticks={marks}
                      interval={0}
                      width={LEFT}
                      axisLine={false}
                      tickLine={false}
                      tickMargin={6}
                      tickFormatter={tidy}
                    />
                    {picked === undefined ? null : (
                      <ReferenceArea
                        className="armada-hours__span"
                        x1={picked.at}
                        x2={picked.to}
                        ifOverflow="hidden"
                      />
                    )}
                    <ReferenceLine className="armada-hours__base" y={0} ifOverflow="visible" />
                    {plot.series.map((line) => (
                      <Line
                        key={line.key}
                        dataKey={line.key}
                        data-model={line.model}
                        data-time={line.time}
                        type="linear"
                        className="armada-hours__line"
                        isAnimationActive={false}
                        activeDot={false}
                        connectNulls={false}
                        dot={(dot: { cx?: number; cy?: number; index: number }) =>
                          line.values[dot.index] !== undefined &&
                          line.values[dot.index - 1] === undefined &&
                          line.values[dot.index + 1] === undefined ? (
                            <circle
                              key={dot.index}
                              className="armada-hours__dot"
                              data-model={line.model}
                              data-time={line.time}
                              cx={dot.cx}
                              cy={dot.cy}
                              r={2}
                            />
                          ) : (
                            <Fragment key={dot.index} />
                          )
                        }
                      />
                    ))}
                    {hour === undefined ? null : (
                      <>
                        <ReferenceArea
                          className="armada-hours__cross"
                          x1={hour}
                          x2={hour + 1}
                          ifOverflow="hidden"
                        />
                        {plot.series.map((line) => {
                          const value = line.values[hour];
                          return value === undefined ? null : (
                            <ReferenceDot
                              key={line.key}
                              x={hour + 0.5}
                              y={value}
                              r={4}
                              ifOverflow="visible"
                              shape={(dot) => (
                                <circle
                                  className="armada-hours__point"
                                  data-model={line.model}
                                  data-time={line.time}
                                  cx={dot.cx}
                                  cy={dot.cy}
                                  r={4}
                                />
                              )}
                            />
                          );
                        })}
                      </>
                    )}
                    {plot.direct === true ? <EndNames series={plot.series} /> : null}
                  </LineChart>
                </ResponsiveContainer>
                {lines(last ? AXIS : 0)}
                {index > 0 || hour === undefined ? null : (
                  <div
                    className="armada-hours__tip"
                    role="status"
                    data-flip={hour >= SHOWN / 2 || undefined}
                    style={{
                      left: along(hour + 0.5),
                    }}
                  >
                    <span className="armada-hours__tipday">{hourName(hours, hour)}</span>
                    {plots.flatMap((each) => {
                      const lines = each.series.flatMap((line) => {
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
                            <span key={one.line.key} className="armada-hours__tipline">
                              <span
                                className={
                                  one.line.model === undefined
                                    ? "armada-time__swatch"
                                    : "armada-usage__swatch"
                                }
                                data-model={one.line.model}
                                data-time={one.line.time}
                                aria-hidden
                              />
                              <span>{one.line.name}</span>
                              <span className="armada-usage__fig">{each.say(one.value)}</span>
                            </span>
                          )),
                        ...(each.sum === undefined || lines.length === 0
                          ? []
                          : [
                              <span key="sum" className="armada-hours__tipline" data-total>
                                <span />
                                <span>{each.sum}</span>
                                <span className="armada-usage__fig">{each.say(sum)}</span>
                              </span>,
                            ]),
                      ];
                    })}
                    {running === undefined ? null : (
                      <span className="armada-hours__tipline" data-total>
                        <span />
                        <span>{commitOf(running.build)}</span>
                        <span className="armada-usage__fig">
                          {running.build.branches.length > 0 ? "preview" : "main"}
                        </span>
                      </span>
                    )}
                  </div>
                )}
              </div>
            </div>
          );
        })}
      </div>
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
  plots: readonly Pick<HourPlot, "series" | "say">[];
  label: string;
}) {
  return (
    <ul className="armada-hours__twin" aria-label={label}>
      {hours.map((one, at) => {
        const said = plots.flatMap((plot) =>
          plot.series.flatMap((line) => {
            const value = line.values[at];
            return value === undefined
              ? []
              : [
                  `${line.name} ${plot.say(value)}`,
                ];
          }),
        );
        return said.length === 0 ? null : (
          <li key={one.at}>{`${hourName(hours, at)}, ${said.join(", ")}`}</li>
        );
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
  return (
    <div className="armada-usage__plot armada-hours" style={frame(false)} aria-hidden>
      <div className="armada-hours__stack">
        {plots.map((plot, index) => (
          <div
            key={index}
            className="armada-hours__plot"
            style={{
              flexGrow: plot.height,
            }}
          >
            {plot.title === undefined ? null : <div className="armada-hours__head" />}
            <div
              className="armada-hours__canvas armada-hours__skel"
              style={
                {
                  minHeight: plot.height + (index === plots.length - 1 ? AXIS : 0),
                  "--crown": px(CROWN),
                  "--foot": px(index === plots.length - 1 ? AXIS : FOOT),
                } as CSSProperties
              }
            >
              <span className="armada-hours__skel-base" />
              {SKEL.map((fill, at) => (
                <Skeleton
                  key={at}
                  className="armada-hours__skel-dash"
                  style={{
                    left: `calc(var(--plot-left) + (100% - var(--plot-left) - var(--plot-right)) * ${(at + 1 / 6) / SKEL.length})`,
                    width: `calc((100% - var(--plot-left) - var(--plot-right)) * ${2 / 3 / SKEL.length})`,
                    bottom: `calc(var(--foot) + (100% - var(--foot) - var(--crown)) * ${fill})`,
                  }}
                />
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
