// The Fleet panel's build section: which build Fleet runs on, where it stands against main, and the
// one act, which follows the selection. Over `scenarios/fleet-build.ts`.
//
// **Every target is one the panel draws only at its resting width**, since under
// `--layout-breakpoint` the panel is a bare head.

import { button, dialog, role, text, walk } from "../walk";

const BUILD = role("combobox", "Build");
const REFRESH = button("Refresh preview");
const UPDATE = button("Update to main");

export const fleetBuildAligned = walk("fleet/build-aligned", [
  { look: BUILD, say: "Main is the build Fleet runs on" },
  { hover: role("img", "Aligned with main", { exact: true }), say: "A check beside main: level with it, and the tooltip says so" },
  { hover: UPDATE, say: "Update to main is there and disabled: On latest main" },
]);

export const fleetBuildBehind = walk("fleet/build-behind", [
  { look: BUILD, say: "Main" },
  { hover: role("img", "3 commits behind main", { exact: true }), say: "An arrow down and a figure: three commits behind, in the warn hue" },
  { hover: UPDATE, say: "Update to main can be pressed: Restart on latest main" },
  { press: UPDATE, say: "Update" },
  { look: text("Fetching main"), say: "The stage the restart is at replaces the figure, with a ring turning beside it" },
  { look: button("Updating to main"), say: "The button says what it is doing and cannot be pressed again" },
]);

export const fleetBuildPreviewAhead = walk("fleet/build-preview-ahead", [
  { look: BUILD, say: "Preview is the build Fleet runs on" },
  { hover: role("img", "4 commits ahead of main", { exact: true }), say: "An arrow up and a figure: four commits ahead of main" },
  { hover: REFRESH, say: "Merge in-flight branches and restart on the preview" },
  { press: REFRESH, say: "Refresh" },
  { look: text("Merging branches"), say: "The stage the restart is at replaces the figure, and moves on about every second: building Fleet, building Bridge, restarting Fleet, reopening Bridge" },
]);

export const fleetBuildRefreshing = walk("fleet/build-refreshing", [
  { look: button("Refreshing preview"), say: "The button is working and cannot be pressed again" },
  { later: text("Merging branches"), say: "The first stage, with a ring turning beside it" },
  { later: text("Building Fleet"), say: "Then the next" },
  { later: text("Building Bridge"), say: "Then the next" },
  { later: text("Restarting Fleet"), say: "Then the next" },
  { look: text("Reopening Bridge"), say: "The last, held until the restart finishes" },
  { hover: role("status"), say: "The tooltip names the build it is restarting onto" },
]);

export const fleetBuildDronesWorking = walk("fleet/build-drones-working", [
  { press: UPDATE, say: "Update to main with two Drones working" },
  { look: dialog("Drones are working"), say: "A confirm lists the Jobs that would be adopted" },
  { look: text("Count drones from the roster"), say: "Each Job by its title" },
  { look: text("Cannot be redirected until it finishes"), say: "The cost of adopting, in one line" },
  { press: button("Restart"), say: "Restart" },
  { look: button("Updating to main"), say: "The restart began once the person said so" },
]);

export const fleetBuildFailed = walk("fleet/build-failed", [
  { look: role("alert"), say: "Why the last restart did not take, in the script's own words" },
  { look: UPDATE, say: "Update to main can be pressed again" },
]);
