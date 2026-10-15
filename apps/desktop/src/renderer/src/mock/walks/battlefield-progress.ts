// A Session's work built on the Cockpit battlefield, twelve hours in eight presses of `later`. One drone builds on the directory it
// writes most; what stands is read off what Fleet recorded. `?build=tower|pad|ring` is how literally it is drawn and
// `?after=stand|clear|monument` what a landed structure does a day on. A mock for the owner to decide the look. #2301.

import { inside, role, walk } from "../walk";

const map = role("listbox", "Map");
const places = role("navigation", "Places");
const dock = inside(map, role("option", "Rebuild the dock panel"));

const battlefieldProgress = walk("battlefield-progress", [
  { look: map, say: "Scope. One repository, three drones. The Session rebuilding the dock panel has read, been shown a walk and asked two questions, and edited nothing. A faint survey grid lies on the apps/desktop plot, with a stake at each corner, and a small marker for the walk and for each question" },
  { later: places, say: "Time passes. It writes a list of seven tasks" },
  { look: map, say: "Plan. A blueprint of the finished thing stands on the plot, one dashed ghost piece for each task. The survey is gone and the markers stay" },
  { later: places, say: "Edits land, four subagents are started, and four tasks are ticked" },
  { look: map, say: "Implement. A task ticked with edits seen since the last tick is a solid piece. One ticked with none seen is outlined and hollow. The next ghost piece has scaffold round it because edits came with no tick. Four small drones circle it while their subagents run, and the Session's laser welds it" },
  { hover: dock, say: "Its card. The phase as a glyph and a word, each task as a swatch drawn as its piece is, solid, outlined or dashed, and nothing on it is counted" },
  { later: places, say: "Ten minutes with no tool call in a running turn. Three subagents end" },
  { look: map, say: "Stalled. The drone stops welding and hangs still under the amber ring. The unfinished part has dimmed. One worker is still flying: its subagent has not ended" },
  { later: places, say: "Thirty more minutes of quiet" },
  { look: map, say: "Forty minutes quiet. The unfinished part is dimmer again" },
  { later: places, say: "It works again and finishes, shows a walk of the built thing and opens a pull request. Its turn ends" },
  { look: map, say: "The road. It runs from the structure to the flag. The walk and the pull request are lit with when they happened. Checks and auto-merge are faint: this repository's Sessions usually do them next" },
  { hover: inside(map, role("option", /^Checks running/)), say: "A faint stop says so, and a lit one says when" },
  { later: places, say: "Checks start and auto-merge is set" },
  { look: map, say: "Every stop is lit" },
  { later: places, say: "It merges" },
  { look: map, say: "Landed. The flag flashes and the structure is complete: a pennant on the tower, the craft lifted off the gantry, the last segment of the ring closed. On `after=clear` it goes after a moment" },
  { later: places, say: "A day later, and Sessions from earlier have landed too" },
  { look: map, say: "A day on. By default finished structures stand small and weathered and fade over a day, the nearest to landing clearest. `after=clear` leaves the plot bare. `after=monument` keeps every one, a small city on the plot" },
]);

export { battlefieldProgress };
