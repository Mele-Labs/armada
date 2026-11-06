import { useId, useState } from "react";
import {
  ArrowDown,
  ArrowUp,
  Check,
  CircleSlash,
  GitCommitHorizontal,
  LoaderCircle,
  RotateCw,
  Undo2,
  X,
  Unplug,
} from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { Dialog } from "../../primitives/Dialog/Dialog";
import { Select } from "../../primitives/Select/Select";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { Line, Mono } from "../PoolSlots/ConfirmLine";

/** The build Fleet runs on: `main` as merged, or the preview, which is main with every in-flight branch merged in. */
export type FleetBuildChoice = "main" | "preview";

/** Commits the running build holds that main lacks, and commits main holds that it lacks. Both zero is aligned. */
export type FleetBuildPosition = {
  ahead: number;
  behind: number;
};

/** Where a restart has got to, in the order it takes them. `merging` is the preview's alone, `fetching_main` main's. */
export type FleetBuildStage =
  | "merging"
  | "fetching_main"
  | "building_fleet"
  | "building_bridge"
  | "restarting_fleet"
  | "reopening_bridge"
  | "retrying";

const STAGE_SAID: Record<FleetBuildStage, string> = {
  merging: "Merging branches",
  fetching_main: "Fetching main",
  building_fleet: "Building Fleet",
  building_bridge: "Building Bridge",
  restarting_fleet: "Restarting Fleet",
  reopening_bridge: "Reopening Bridge",
  retrying: "Retrying build",
};

/** A Job whose Drone is working, and so is adopted by a restart. */
export type FleetBuildDrone = {
  id: string;
  label: string;
};

/** A branch the last Refresh preview did not merge in, and the script's reason in one line. `moved` is a branch with commits since; absent is false. */
export type FleetBuildLeftOut = {
  branch: string;
  why: string;
  moved?: boolean;
};

/** One build kept on disk, as the panel names it. */
export type FleetBuildKept = {
  /** When it was built: `5 Nov, 16:20`. */
  built: string;
  /** Its short commit. */
  commit: string;
};

/** The build before this one, which Revert swaps Fleet and Bridge back onto, and what that changes. */
export type FleetBuildPrevious = FleetBuildKept & {
  /** Its Bridge differs from the one open, so this window closes and opens on it. */
  bridgeReopens: boolean;
  /** The Jobs whose change the running build holds and this one does not. */
  takesOut: readonly string[];
  /** The Jobs whose change this one holds and the running build does not. */
  bringsBack: readonly string[];
};

/** A Job's branch picked for the snapshot: named by its Job's title, or by the branch where no Job holds it. */
export type FleetBuildPicked = {
  branch: string;
  title?: string;
};

/** The snapshot of picked Jobs, built beside the running build and kept without swapping. */
export type FleetBuildSnapshot = {
  state: "building" | "ready" | "failed";
  /** Where a build under way has got to. Absent until it says. */
  stage?: FleetBuildStage;
  /** The Jobs it holds, by title. Drawn when ready. */
  jobs: readonly string[];
  /** Swapping onto it closes this window and opens it on the snapshot's. */
  bridgeReopens: boolean;
  /** Why it did not build, in one line. */
  failed?: string;
};

export type FleetBuild = {
  on: FleetBuildChoice;
  /** Absent where it cannot be counted, which draws no mark: that is not aligned. */
  position?: FleetBuildPosition;
  /** The build Fleet is restarting onto, or none. */
  working: FleetBuildChoice | null;
  /** Where that restart has got to. Absent until it says, which draws the ring alone. */
  stage?: FleetBuildStage;
  /** The Jobs a restart would adopt. Any of them asks first; none restarts at once. */
  drones: readonly FleetBuildDrone[];
  /** Why the last restart did not take, in one line. */
  failed?: string;
  /** The in-flight branches the preview was built without. Drawn only on the preview, with nothing restarting. */
  leftOut?: readonly FleetBuildLeftOut[];
  /** Restart onto the other build. `adopt` is the person's say-so past a working Drone. */
  onChoose: (build: FleetBuildChoice, adopt: boolean) => void;
  /** Restart onto the build in use: merge the preview again, or move to the latest main. */
  onRestart: (adopt: boolean) => void;
  /** The build Fleet runs now, where it is a kept one. */
  running?: FleetBuildKept;
  /** The build Revert swaps onto. Absent draws no Revert. */
  previous?: FleetBuildPrevious;
  /** A revert is under way. */
  reverting?: boolean;
  /** The Jobs with work running, which their keepers carry across the restart. */
  carried?: readonly string[];
  /** Swap Fleet and Bridge back onto `previous`. */
  onRevert?: () => void;
  /** The branches picked for the next snapshot. None draws no chips. */
  picked?: readonly FleetBuildPicked[];
  /** The snapshot of picked Jobs. Absent draws no row. */
  snapshot?: FleetBuildSnapshot;
  /** A swap onto the snapshot is under way. */
  swapping?: boolean;
  /** Take a branch out of the picked Jobs. */
  onUnpick?: (branch: string) => void;
  /** Swap Fleet and Bridge onto the snapshot. */
  onSwap?: () => void;
};

const keptSaid = (kept: FleetBuildKept) => `${kept.built} · ${kept.commit}`;

/** One row of the Revert confirm: what it is about, then a line or the Jobs it names, and why under them. */
function RevertRow({
  label,
  value,
  jobs,
  why,
}: {
  label: string;
  value?: string;
  jobs?: readonly string[];
  why?: string;
}) {
  return (
    <>
      <dt className="armada-fleet-build__revert-label">{label}</dt>
      <dd className="armada-fleet-build__revert-value">
        {jobs === undefined ? (
          value
        ) : (
          <ul aria-label={label}>
            {jobs.map((one) => (
              <li key={one}>{one}</li>
            ))}
          </ul>
        )}
        {why === undefined ? null : <p className="armada-fleet-build__revert-why">{why}</p>}
      </dd>
    </>
  );
}

const commits = (count: number) => (count === 1 ? "1 commit" : `${count} commits`);

const MOVED_SAID = "Moved since it was left out";

const aheadSaid = (count: number) => `${commits(count)} ahead of main`;
const behindSaid = (count: number) => `${commits(count)} behind main`;

/** What a restart costs a Drone that is working, on hover; the line beside it says the one that matters most. */
const ADOPTING_COSTS =
  "Its pipes die: no redirect, poke or verdict. Its recorded spend is an undercount. It shows as unheard. " +
  "Its servers stop with Fleet. A Check running mid-gate most likely re-runs from scratch";

/** The act the person pressed, held while the Jobs it would adopt are shown. */
type Asked =
  | {
      kind: "choose";
      build: FleetBuildChoice;
    }
  | {
      kind: "restart";
    };

/**
 * Where the running build stands against main, the build it runs on, and the
 * one act on it. **The button follows the selection**: on Preview it merges the
 * in-flight branches again and restarts, on Main it moves Fleet to the latest
 * main, and level with main it is disabled rather than absent. **The position is
 * a glyph and a figure with the words on hover**, not a phrase: an arrow up for
 * commits main lacks, down for commits it has that the build does not, a check
 * when there are none of either. Status hue is mapped, never chosen: behind main
 * is the amber the panel's Doctor line uses for a warn, aligned the green.
 */
export function FleetBuildSection({ build }: { build: FleetBuild }) {
  const { on, position, working, stage, drones } = build;
  const [asked, setAsked] = useState<Asked | null>(null);
  const [revertAsked, setRevertAsked] = useState(false);
  const pickedLabel = useId();
  const { previous } = build;
  const reverting = build.reverting === true;
  const swapping = build.swapping === true;
  const { picked, snapshot } = build;
  const carried = build.carried ?? [];
  const aligned = position !== undefined && position.ahead === 0 && position.behind === 0;
  // The button names what is being restarted onto while that is under way, and the build in use otherwise.
  const target = working ?? on;
  const idleOnMain = working === null && on === "main" && aligned;
  const label =
    target === "preview"
      ? working === null
        ? "Refresh preview"
        : "Refreshing preview"
      : working === null
        ? "Update to main"
        : "Updating to main";
  const tip = idleOnMain
    ? "On latest main"
    : target === "preview"
      ? "Merge in-flight branches and restart on the preview"
      : "Restart on latest main";
  const press = (act: Asked) => {
    if (drones.length > 0) setAsked(act);
    else send(act, false);
  };
  const send = (act: Asked, adopt: boolean) => {
    if (act.kind === "choose") build.onChoose(act.build, adopt);
    else build.onRestart(adopt);
  };
  return (
    <div className="armada-fleet-build">
      <Tooltip label="Build Fleet runs on">
        <Select
          aria-label="Build"
          value={on}
          disabled={working !== null}
          onChange={(event) => {
            const chosen = event.target.value === "preview" ? "preview" : "main";
            if (chosen !== on)
              press({
                kind: "choose",
                build: chosen,
              });
          }}
        >
          <option value="main">Main</option>
          <option value="preview">Preview</option>
        </Select>
      </Tooltip>
      <Tooltip label={tip}>
        <Button
          variant="secondary"
          size="sm"
          pending={working !== null}
          disabled={idleOnMain || working !== null || reverting || swapping}
          onClick={() =>
            press({
              kind: "restart",
            })
          }
        >
          {working !== null ? (
            <LoaderCircle size={16} strokeWidth={2} aria-hidden="true" />
          ) : (
            <RotateCw size={16} strokeWidth={2} aria-hidden="true" />
          )}
          {label}
        </Button>
      </Tooltip>
      {previous === undefined ? null : (
        <Tooltip label={`Back to the build of ${previous.built}, ${previous.commit}`}>
          <Button
            variant="secondary"
            size="sm"
            pending={reverting}
            disabled={working !== null || reverting || swapping}
            onClick={() => setRevertAsked(true)}
          >
            {reverting ? (
              <LoaderCircle size={16} strokeWidth={2} aria-hidden="true" />
            ) : (
              <Undo2 size={16} strokeWidth={2} aria-hidden="true" />
            )}
            {reverting ? "Reverting" : "Revert"}
          </Button>
        </Tooltip>
      )}
      {picked === undefined || picked.length === 0 ? null : (
        <>
          <span className="armada-fleet-build__label" id={pickedLabel}>
            Next build holds
          </span>
          <ul className="armada-fleet-build__picked" aria-labelledby={pickedLabel}>
            {picked.map((one) => {
              const name = one.title ?? one.branch;
              return (
                <li key={one.branch} className="armada-fleet-build__chip">
                  <span className="armada-fleet-build__chip-name">{name}</span>
                  <Tooltip label="Leave out of the build">
                    <Button
                      variant="ghost"
                      size="sm"
                      iconOnly
                      aria-label={`Leave ${name} out of the build`}
                      onClick={() => build.onUnpick?.(one.branch)}
                    >
                      <X size={12} strokeWidth={2} aria-hidden="true" />
                    </Button>
                  </Tooltip>
                </li>
              );
            })}
          </ul>
        </>
      )}
      {snapshot === undefined ? null : snapshot.state === "building" ? (
        <p className="armada-fleet-build__snapshot" role="status">
          <LoaderCircle
            className="armada-fleet-build__spin"
            size={12}
            strokeWidth={2}
            aria-hidden="true"
          />
          Building
          {snapshot.stage === undefined ? null : (
            <span className="armada-fleet-build__stage">{STAGE_SAID[snapshot.stage]}</span>
          )}
        </p>
      ) : snapshot.state === "failed" ? (
        <p className="armada-fleet-build__failed" role="alert">
          {snapshot.failed}
        </p>
      ) : (
        <div className="armada-fleet-build__ready">
          <span className="armada-fleet-build__mark" data-tone="success">
            <Check size={12} strokeWidth={2} aria-hidden="true" />
            Ready
          </span>
          {snapshot.jobs.length === 0 ? null : (
            <ul className="armada-fleet-build__ready-jobs" aria-label="Held by the snapshot">
              {snapshot.jobs.map((one) => (
                <li key={one}>{one}</li>
              ))}
            </ul>
          )}
          {snapshot.bridgeReopens ? (
            <p className="armada-fleet-build__reopens">Bridge reopens</p>
          ) : null}
          <Button
            variant="secondary"
            size="sm"
            pending={swapping}
            disabled={swapping || reverting || working !== null}
            onClick={() => build.onSwap?.()}
          >
            {swapping ? "Swapping" : "Swap"}
          </Button>
        </div>
      )}
      {build.failed === undefined || working !== null ? null : (
        <p className="armada-fleet-build__failed" role="alert">
          {build.failed}
        </p>
      )}
      {on !== "preview" ||
      working !== null ||
      build.leftOut === undefined ||
      build.leftOut.length === 0 ? null : (
        <ul className="armada-fleet-build__left-out" aria-label="Left out of the preview">
          {build.leftOut.map((one) => (
            <li key={one.branch} className="armada-fleet-build__left-out-row">
              <Tooltip label="Left out of the preview">
                <span
                  className="armada-fleet-build__mark"
                  data-tone="warn"
                  role="img"
                  aria-label="Left out of the preview"
                >
                  <CircleSlash size={12} strokeWidth={2} aria-hidden="true" />
                </span>
              </Tooltip>
              <span className="armada-fleet-build__branch">{one.branch}</span>
              {/* Drawn empty on an unmoved branch, so the column holds its width and the rows line up. */}
              <span className="armada-fleet-build__moved">
                {one.moved === true ? (
                  <Tooltip label={MOVED_SAID}>
                    <span className="armada-fleet-build__mark" role="img" aria-label={MOVED_SAID}>
                      <GitCommitHorizontal size={12} strokeWidth={2} aria-hidden="true" />
                    </span>
                  </Tooltip>
                ) : null}
              </span>
              <Tooltip label={one.why}>
                <span className="armada-fleet-build__why">{one.why}</span>
              </Tooltip>
            </li>
          ))}
        </ul>
      )}
      <div className="armada-fleet-build__position">
        {working === null ? <span className="armada-fleet-build__label">main</span> : null}
        {working !== null ? (
          <Tooltip label={`Restarting Fleet on ${target === "preview" ? "the preview" : "main"}`}>
            <span
              className="armada-fleet-build__mark"
              data-tone="muted"
              {...(stage === undefined
                ? {
                    role: "img",
                    "aria-label": "Restarting Fleet",
                  }
                : {
                    role: "status",
                  })}
            >
              <LoaderCircle
                className="armada-fleet-build__spin"
                size={12}
                strokeWidth={2}
                aria-hidden="true"
              />
              {stage === undefined ? null : STAGE_SAID[stage]}
            </span>
          </Tooltip>
        ) : position === undefined ? null : (
          <span className="armada-fleet-build__marks">
            {aligned ? (
              <Tooltip label="Aligned with main">
                <span
                  className="armada-fleet-build__mark"
                  data-tone="success"
                  role="img"
                  aria-label="Aligned with main"
                >
                  <Check size={12} strokeWidth={2} aria-hidden="true" />
                </span>
              </Tooltip>
            ) : (
              <>
                {position.ahead === 0 ? null : (
                  <Tooltip label={aheadSaid(position.ahead)}>
                    <span
                      className="armada-fleet-build__mark"
                      data-tone="muted"
                      role="img"
                      aria-label={aheadSaid(position.ahead)}
                    >
                      <ArrowUp size={12} strokeWidth={2} aria-hidden="true" />
                      {position.ahead}
                    </span>
                  </Tooltip>
                )}
                {position.behind === 0 ? null : (
                  <Tooltip label={behindSaid(position.behind)}>
                    <span
                      className="armada-fleet-build__mark"
                      data-tone="warn"
                      role="img"
                      aria-label={behindSaid(position.behind)}
                    >
                      <ArrowDown size={12} strokeWidth={2} aria-hidden="true" />
                      {position.behind}
                    </span>
                  </Tooltip>
                )}
              </>
            )}
          </span>
        )}
      </div>
      {!revertAsked || previous === undefined ? null : (
        <Dialog
          open
          tone="neutral"
          title={`Revert to the build of ${previous.built}`}
          confirmLabel="Revert"
          onCancel={() => setRevertAsked(false)}
          onConfirm={() => {
            build.onRevert?.();
            setRevertAsked(false);
          }}
        >
          <dl className="armada-fleet-build__revert">
            {build.running === undefined ? null : (
              <RevertRow label="Running now" value={keptSaid(build.running)} />
            )}
            <RevertRow label="Goes back to" value={keptSaid(previous)} />
            {previous.takesOut.length === 0 ? null : (
              <RevertRow
                label="Jobs taken out"
                jobs={previous.takesOut}
                why={`Their changes are in the ${build.running?.built ?? "running"} build and not in the ${previous.built} one, so your app goes without them. The Jobs themselves carry on.`}
              />
            )}
            {previous.bringsBack.length === 0 ? null : (
              <RevertRow
                label="Jobs brought back"
                jobs={previous.bringsBack}
                why={`Their changes are in the ${previous.built} build and not in the ${build.running?.built ?? "running"} one, so your app gets them back.`}
              />
            )}
            <RevertRow
              label="Restarts"
              value={
                previous.bridgeReopens ? "Fleet, and this window" : "Fleet. This window stays open"
              }
            />
            {carried.length === 0 ? null : (
              <RevertRow
                label="Jobs still working"
                jobs={carried}
                why="These Jobs are at work now. Their agents keep running while Fleet restarts."
              />
            )}
          </dl>
        </Dialog>
      )}
      {asked === null ? null : (
        <Dialog
          open
          tone="neutral"
          title="Drones are working"
          confirmLabel="Restart"
          onCancel={() => setAsked(null)}
          onConfirm={() => {
            send(asked, true);
            setAsked(null);
          }}
        >
          <Line Glyph={Unplug} said={ADOPTING_COSTS} word="Cannot be redirected until it finishes">
            <Mono label="Jobs adopted" items={drones.map((one) => one.label)} />
          </Line>
        </Dialog>
      )}
    </div>
  );
}
