// The Cockpit's flight controls as one console, Dispatch in its middle, in three takes picked by `?take=arc|wrap|dash`
// (`cockpit/deck/console.tsx`). With no Job open. Over `dashboard-quick-dispatch`.

import { button, inside, region, role, walk } from "../walk";

const controls = region("Flight controls");
const frame = role("region", "Dispatch", { exact: true });

const key = (name: string) => inside(controls, button(name));

const cockpitConsole = walk("dashboard-quick-dispatch", [
  { look: controls, say: "Flight control as one console. Dispatch is the middle key" },
  { key: "Home", on: key("Checks"), say: "Home, the first key. Tab reaches it too" },
  { key: "ArrowRight", on: key("Checks"), say: "Right arrow, the next key" },
  { key: "ArrowRight", on: key("Needs me"), say: "Right arrow again, Dispatch" },
  { key: "ArrowLeft", on: key("Dispatch"), say: "Left arrow, back one" },
  { key: "ArrowRight", on: key("Needs me"), say: "Right arrow, Dispatch" },
  { key: "Enter", on: key("Dispatch"), say: "Enter presses it" },
  { look: inside(frame, role("textbox", "Request")), say: "The Dispatch sheet" },
  { key: "Escape", on: inside(frame, role("textbox", "Request")), say: "Escape closes it" },
]);

export { cockpitConsole as "cockpit-console" };
