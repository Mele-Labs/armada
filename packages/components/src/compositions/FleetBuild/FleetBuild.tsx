import { useState } from "react";
import { ArrowDown, ArrowUp, Check, LoaderCircle, RotateCw, Unplug } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { Dialog } from "../../primitives/Dialog/Dialog";
import { Select } from "../../primitives/Select/Select";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { Line, Mono } from "../PoolSlots/ConfirmLine";

/** The build Fleet runs on: `main` as merged, or the preview, which is main with every in-flight branch merged in. */
export type FleetBuildChoice = "main" | "preview";

/** Commits the running build holds that main lacks, and commits main holds that it lacks. Both zero is aligned. */
export type FleetBuildPosition = { ahead: number; behind: number };

/** A Job whose Drone is working, and so is adopted by a restart. */
export type FleetBuildDrone = { id: string; label: string };

export type FleetBuild = {
  on: FleetBuildChoice;
  /** Absent where it cannot be counted, which draws no mark: that is not aligned. */
  position?: FleetBuildPosition;
  /** The build Fleet is restarting onto, or none. */
  working: FleetBuildChoice | null;
  /** The Jobs a restart would adopt. Any of them asks first; none restarts at once. */
  drones: readonly FleetBuildDrone[];
  /** Why the last restart did not take, in one line. */
  failed?: string;
  /** Restart onto the other build. `adopt` is the person's say-so past a working Drone. */
  onChoose: (build: FleetBuildChoice, adopt: boolean) => void;
  /** Restart onto the build in use: merge the preview again, or move to the latest main. */
  onRestart: (adopt: boolean) => void;
};

const commits = (count: number) => (count === 1 ? "1 commit" : `${count} commits`);

const aheadSaid = (count: number) => `${commits(count)} ahead of main`;
const behindSaid = (count: number) => `${commits(count)} behind main`;

/** What a restart costs a Drone that is working, on hover; the line beside it says the one that matters most. */
const ADOPTING_COSTS =
  "Its pipes die: no redirect, poke or verdict. Its recorded spend is an undercount. It shows as unheard. " +
  "Its servers stop with Fleet. A Check running mid-gate most likely re-runs from scratch";

/** The act the person pressed, held while the Jobs it would adopt are shown. */
type Asked = { kind: "choose"; build: FleetBuildChoice } | { kind: "restart" };

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
  const { on, position, working, drones } = build;
  const [asked, setAsked] = useState<Asked | null>(null);
  const aligned = position !== undefined && position.ahead === 0 && position.behind === 0;
  // The button names what is being restarted onto while that is under way, and the build in use otherwise.
  const target = working ?? on;
  const idleOnMain = working === null && on === "main" && aligned;
  const label = target === "preview" ? (working === null ? "Refresh preview" : "Refreshing preview") : working === null ? "Update to main" : "Updating to main";
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
            if (chosen !== on) press({ kind: "choose", build: chosen });
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
          disabled={idleOnMain || working !== null}
          onClick={() => press({ kind: "restart" })}
        >
          {working !== null ? (
            <LoaderCircle size={16} strokeWidth={2} aria-hidden="true" />
          ) : (
            <RotateCw size={16} strokeWidth={2} aria-hidden="true" />
          )}
          {label}
        </Button>
      </Tooltip>
      {build.failed === undefined || working !== null ? null : (
        <p className="armada-fleet-build__failed" role="alert">
          {build.failed}
        </p>
      )}
      <div className="armada-fleet-build__position">
        <span className="armada-fleet-build__label">main</span>
        {working !== null ? (
          <Tooltip label={`Restarting Fleet on ${target === "preview" ? "the preview" : "main"}`}>
            <span className="armada-fleet-build__mark" data-tone="muted" role="img" aria-label="Restarting Fleet">
              <LoaderCircle className="armada-fleet-build__spin" size={12} strokeWidth={2} aria-hidden="true" />
            </span>
          </Tooltip>
        ) : position === undefined ? null : (
          <span className="armada-fleet-build__marks">
            {aligned ? (
              <Tooltip label="Aligned with main">
                <span className="armada-fleet-build__mark" data-tone="success" role="img" aria-label="Aligned with main">
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
