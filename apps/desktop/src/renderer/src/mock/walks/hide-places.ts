// The places in the notch hidden and brought back by a right press, and the same list under System → Layout (the owner, 26 Oct 2026).
// Over the `pr-review` scenario, which draws Reviews.

import { inside, place, role, toSystemPage, walk } from "../walk";

const placesList = role("group", "Places");

const hidePlaces = walk("pr-review", [
  { context: place("Studios"), say: "Right-click a place" },
  { press: role("menuitem", "Hide Studios"), say: "Hide Studios" },
  { key: "Meta+2", on: role("navigation", "Places"), say: "Studios is off the notch, and ⌘2 is Logbook now" },
  { context: place("Reviews"), say: "Right-click another place. View lists every place" },
  { hover: role("menuitem", "System", { exact: true }), say: "Cockpit and System are always shown" },
  { press: role("menuitem", "Studios", { exact: true }), say: "Studios back on. The menu stays open" },
  { key: "Escape", on: role("menu"), say: "Esc closes it" },
  ...toSystemPage("Layout", "The same list in System, under Layout"),
  { look: placesList, say: "Places, each with its switch" },
  { press: inside(placesList, role("switch", "Logbook")), say: "Logbook off here" },
  { look: role("navigation", "Places"), say: "And off the notch" },
]);

export { hidePlaces as "hide-places" };
