// Where the terminal pane opens, which the two takes of it differ in. A pick the window remembers, as
// the Cockpit's timeline take is, and a walk sets it before the window mounts (`Walk.remembering`).

export const PLACE_KEY = "armada.bridge.terminal-place";

export type Place = "inline" | "docked";

/** Under the block that was run until the other take is picked. */
export function placeOf(): Place {
  try {
    return window.localStorage.getItem(PLACE_KEY) === "docked" ? "docked" : "inline";
  } catch {
    return "inline";
  }
}
