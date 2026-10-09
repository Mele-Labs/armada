import { ArrowDown, ArrowUp, Check, LoaderCircle, RotateCw } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { Select } from "../../primitives/Select/Select";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/** The build Fleet runs on: `main` as merged, or the preview, which is main with every in-flight branch merged in. */
export type FleetBuildChoice = "main" | "preview";

/** Commits the running build holds that main lacks, and commits main holds that it lacks. Both zero is aligned. */
export type FleetBuildPosition = { ahead: number; behind: number };

/** What the running build is doing instead of running: a restart onto the other build, or onto a refreshed preview. */
export type FleetBuildWork = "switching" | "refreshing";

export type FleetBuild = {
  on: FleetBuildChoice;
  position: FleetBuildPosition;
  working: FleetBuildWork | null;
  onChoose: (build: FleetBuildChoice) => void;
  onRefresh: () => void;
};

const commits = (count: number) => (count === 1 ? "1 commit" : `${count} commits`);

const aheadSaid = (count: number) => `${commits(count)} ahead of main`;
const behindSaid = (count: number) => `${commits(count)} behind main`;

/**
 * Where the running build stands against main, the build it runs on, and the
 * one act on the preview. **The position is a glyph and a figure with the
 * words on hover**, not a phrase: an arrow up for commits main lacks, down for
 * commits it has that the build does not, a check when there are none of
 * either. Status hue is mapped, never chosen: behind main is the amber the
 * panel's Doctor line uses for a warn, aligned the green.
 */
export function FleetBuildSection({ build }: { build: FleetBuild }) {
  const { on, position, working } = build;
  const aligned = position.ahead === 0 && position.behind === 0;
  const refreshing = working === "refreshing";
  const refreshTip = on === "preview"
    ? "Merge every in-flight branch into the preview and restart Fleet on it. Working Drones are adopted and cannot be redirected until they finish."
    : "Select Preview first";
  return (
    <div className="armada-fleet-build">
      <Tooltip label="Build Fleet runs on">
        <Select
          aria-label="Build"
          value={on}
          disabled={working !== null}
          onChange={(event) => build.onChoose(event.target.value === "preview" ? "preview" : "main")}
        >
          <option value="main">Main</option>
          <option value="preview">Preview</option>
        </Select>
      </Tooltip>
      <Tooltip label={refreshTip}>
        <Button
          variant="secondary"
          size="sm"
          pending={refreshing}
          disabled={on !== "preview" || working !== null}
          onClick={build.onRefresh}
        >
          {refreshing ? (
            <LoaderCircle size={16} strokeWidth={2} aria-hidden="true" />
          ) : (
            <RotateCw size={16} strokeWidth={2} aria-hidden="true" />
          )}
          {refreshing ? "Refreshing preview" : "Refresh preview"}
        </Button>
      </Tooltip>
      <div className="armada-fleet-build__position">
        <span className="armada-fleet-build__label">main</span>
        {working !== null ? (
          <Tooltip label={`Restarting Fleet on ${on === "preview" ? "the preview" : "main"}`}>
            <span className="armada-fleet-build__mark" data-tone="muted" role="img" aria-label="Restarting Fleet">
              <LoaderCircle className="armada-fleet-build__spin" size={12} strokeWidth={2} aria-hidden="true" />
            </span>
          </Tooltip>
        ) : (
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
    </div>
  );
}
