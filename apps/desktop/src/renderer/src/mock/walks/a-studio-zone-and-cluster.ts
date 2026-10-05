// A Studio's Zone and Cluster in the frame language the approval canvas's
// lanes share (4 Oct 2026): a Zone walled, its head a strip naming it; a
// Cluster dashed, its kind beside its title.

import { button, role, walk } from "../walk";

export const aStudioZoneAndCluster = walk("studio-zone", [
  { press: button("Studios", { exact: true }), say: "This repository's Studios" },
  { press: button("Open", { exact: true }), say: "The Studio the issue was read into" },
  { look: role("group", "Zone", { exact: true }), say: "A Zone: walled, its head a strip" },
  { look: role("group", "Cluster: The problem today"), say: "A Cluster: dashed, its kind beside its title" },
]);
