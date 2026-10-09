// The Fleet panel's build section: which build Fleet runs on, where it stands against main, and
// the one act on the preview. The Stats panel above it is gone. Over `scenarios/fleet-build.ts`.
//
// **Every target is one the panel draws only at its resting width**, since under
// `--layout-breakpoint` the panel is a bare head.

import { button, role, walk } from "../walk";

const BUILD = role("combobox", "Build");
const REFRESH = button(/Refresh/);

export const fleetBuildAligned = walk("fleet/build-aligned", [
  { look: BUILD, say: "Main is the build Fleet runs on" },
  { hover: role("img", "Aligned with main", { exact: true }), say: "A check beside main: level with it, and the tooltip says so" },
  { look: REFRESH, say: "Refresh preview is there and cannot be pressed on Main" },
]);

export const fleetBuildBehind = walk("fleet/build-behind", [
  { look: BUILD, say: "Main" },
  { hover: role("img", "3 commits behind main", { exact: true }), say: "An arrow down and a figure: three commits behind, in the warn hue" },
]);

export const fleetBuildPreviewAhead = walk("fleet/build-preview-ahead", [
  { look: BUILD, say: "Preview is the build Fleet runs on" },
  { hover: role("img", "4 commits ahead of main", { exact: true }), say: "An arrow up and a figure: four commits ahead of main" },
  { hover: REFRESH, say: "What a refresh does, and what it costs a working Drone" },
  { press: REFRESH, say: "Refresh" },
  { look: role("img", "Restarting Fleet", { exact: true }), say: "The ring turns where the figure was, and the button shows it is working" },
]);

export const fleetBuildRefreshing = walk("fleet/build-refreshing", [
  { look: button("Refreshing preview"), say: "The button is working and cannot be pressed again" },
  { hover: role("img", "Restarting Fleet", { exact: true }), say: "The ring beside main says what is under way" },
]);
