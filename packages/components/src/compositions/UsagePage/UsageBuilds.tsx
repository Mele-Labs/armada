import { ArrowDown, ArrowUp, GitMerge } from "lucide-react";
import { useEffect, useState } from "react";
import type { KeyboardEvent } from "react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { Band, Bar, Fig } from "./usage-parts";
import {
  FIGURE_HEAD,
  FIGURE_MEANS,
  FIGURE_NAME,
  TIME_FIGURES,
  TOO_FEW,
  buildTip,
  changeOf,
  shown,
  startedAt,
} from "./usage-builds";
import type { Build, Figure, UsageBuilds } from "./usage-builds";

/**
 * The builds the figures are read against, newest first, one row each: its commit, when it began, and the view's figures
 * with each one's change against the build before. A preview build carries a glyph naming what it holds. A build with too
 * few turns to compare shows its figures dimmed and no change. Picking a row narrows the page's other panels to that build.
 *
 * Keys: `c` moves focus here, `↑↓ j k` move down the builds, `↵ o` picks the one under the cursor, `Esc` clears the pick.
 */

/** The figures a view draws: burn per turn on Tokens, and the four times on Time. */
const FIGURES: Readonly<Record<"tokens" | "time", readonly Figure[]>> = {
  tokens: [
    "burn",
  ],
  time: TIME_FIGURES,
};

type Move = ReturnType<typeof changeOf>;

/** A figure's change as the page's delta chip. Where nothing moved the slot stays empty. */
function Change({ move, figure }: { move: Move; figure: Figure }) {
  if (move === undefined || move.still) return <span className="armada-builds__chip" />;
  const tip = `${FIGURE_NAME[figure]} ${move.up ? "up" : "down"} ${Math.round(move.percent)} percent on the build before`;
  return (
    <span className="armada-builds__chip">
      <Tooltip label={tip}>
        <span className="armada-usage__delta" data-way={move.up ? "worse" : "better"} aria-label={tip}>
          {move.up ? <ArrowUp size={12} aria-hidden /> : <ArrowDown size={12} aria-hidden />}
          {`${Math.round(move.percent)}%`}
        </span>
      </Tooltip>
    </span>
  );
}

function BuildRow({
  build,
  before,
  figures,
  cursor,
  picked,
  onPick,
}: {
  build: Build;
  before: Build | undefined;
  figures: readonly Figure[];
  cursor: boolean;
  picked: boolean;
  onPick: () => void;
}) {
  const few = build.turns < TOO_FEW;
  const preview = build.branches.length > 0;
  const fewTip = `${build.turns} turns, too few to compare`;
  const day = startedAt(build.started);
  const moves = figures.map((one) => changeOf(build, before, one));
  const said = figures.map((one, at) => {
    const move = moves[at];
    const how =
      move === undefined || move.still
        ? ""
        : `, ${move.up ? "up" : "down"} ${Math.round(move.percent)} percent`;
    return `${FIGURE_NAME[one]} ${shown(build, one)}${how}`;
  });
  return (
    <li
      role="option"
      id={`build-${build.id}`}
      aria-selected={picked}
      aria-label={`${build.id}, ${day}${preview ? ", preview" : ", main"}, ${said.join(", ")}${few ? `, ${fewTip}` : ""}`}
      data-cursor={cursor || undefined}
      data-few={few || undefined}
      className="armada-builds__row"
      onClick={onPick}
    >
      <span className="armada-builds__kind">
        {preview ? (
          <Tooltip label={buildTip(build)}>
            <span className="armada-builds__glyph">
              <GitMerge size={12} aria-hidden />
            </span>
          </Tooltip>
        ) : null}
      </span>
      {preview ? (
        <span className="armada-builds__commit">{build.id}</span>
      ) : (
        <Tooltip label={buildTip(build)}>
          <span className="armada-builds__commit">{build.id}</span>
        </Tooltip>
      )}
      <span className="armada-builds__started">{day}</span>
      {figures.map((one, at) => (
        <span key={one} className="armada-builds__cell">
          <Fig tip={few ? `${FIGURE_MEANS[one]}. ${fewTip}` : FIGURE_MEANS[one]}>{shown(build, one)}</Fig>
          <Change move={moves[at]} figure={one} />
        </span>
      ))}
    </li>
  );
}

function PendingBuilds({ view }: { view: "tokens" | "time" }) {
  const count = FIGURES[view].length;
  return (
    <ul className="armada-builds__rows" data-view={view} aria-hidden>
      {Array.from(
        {
          length: 8,
        },
        (_, at) => (
          <li key={at} className="armada-builds__row">
            <span className="armada-builds__kind" />
            <Bar width="80%" />
            <Bar width="80%" />
            {Array.from(
              {
                length: count,
              },
              (_, cell) => (
                <span key={cell} className="armada-builds__cell">
                  <Bar width="60%" end />
                  <span className="armada-builds__chip" />
                </span>
              ),
            )}
          </li>
        ),
      )}
    </ul>
  );
}

type Props = {
  /** `pending` draws the shape of what is coming. */
  builds: UsageBuilds | "pending";
  view: "tokens" | "time";
  /** The picked build's id. */
  pick: string | undefined;
  onPick: (id: string) => void;
};

export function BuildsPanel({ builds, view, pick, onPick }: Props) {
  const [cursor, setCursor] = useState(-1);
  const ready = builds === "pending" ? undefined : builds;
  const count = ready?.length ?? 0;
  const figures = FIGURES[view];
  const active = ready?.[cursor];

  useEffect(() => {
    if (active !== undefined)
      document.getElementById(`build-${active.id}`)?.scrollIntoView?.({
        block: "nearest",
      });
  }, [
    active,
  ]);

  const pressed = (event: KeyboardEvent<HTMLUListElement>) => {
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    if (event.key === "ArrowDown" || event.key === "j") setCursor((at) => Math.min(count - 1, at + 1));
    else if (event.key === "ArrowUp" || event.key === "k") setCursor((at) => Math.max(0, at - 1));
    else if ((event.key === "Enter" || event.key === "o") && active !== undefined) onPick(active.id);
    else return;
    event.preventDefault();
  };

  return (
    <section className="armada-usage__panel" aria-label="Builds">
      <Band
        keys={[
          "c",
          "↑↓",
          "↵",
          "Esc",
        ]}
      >
        Builds
      </Band>
      {ready === undefined ? (
        <PendingBuilds view={view} />
      ) : (
        <>
          <div className="armada-builds__heads" data-view={view} aria-hidden>
            <span />
            <span>Build</span>
            <span>Started</span>
            {figures.map((one) => (
              <span key={one} className="armada-builds__head">
                <Tooltip label={FIGURE_MEANS[one]}>
                  <span>{FIGURE_HEAD[one]}</span>
                </Tooltip>
              </span>
            ))}
          </div>
          <ul
            className="armada-builds__rows"
            data-builds
            data-view={view}
            role="listbox"
            aria-label="Builds"
            aria-activedescendant={active === undefined ? undefined : `build-${active.id}`}
            tabIndex={0}
            onKeyDown={pressed}
            onFocus={() => setCursor((at) => (at >= 0 ? at : Math.max(0, ready.findIndex((one) => one.id === pick))))}
            onBlur={() => setCursor(-1)}
          >
            {ready.map((one, at) => (
              <BuildRow
                key={one.id}
                build={one}
                before={ready[at + 1]}
                figures={figures}
                cursor={at === cursor}
                picked={one.id === pick}
                onPick={() => {
                  setCursor(at);
                  onPick(one.id);
                }}
              />
            ))}
          </ul>
        </>
      )}
    </section>
  );
}
