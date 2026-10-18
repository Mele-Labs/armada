// Flight control, with facts on its tiles and edited in place: a Job's tiles carry the context fill, a lit Auto land and Pause, a pulsing Checks and
// the subagents running; Edit reorders by keys, hides a tile, adds a Words and a Shell action and changes Compact's command; a Shell action's
// output stands where the tiles were; the tiles with no Job open are edited the same way, with Page as their one kind. The owner's ask of 18 Oct 2026.

import { button, dialog, inside, region, role, walk } from "../walk";

const list = region("Jobs", { exact: true });
const controls = region("Flight controls");
const tile = (name: string) => inside(controls, button(name));
const editing = inside(controls, role("listbox", "Flight controls, editing"));
const option = (name: string) => inside(editing, role("option", name));
const sheet = (name: string) => dialog(name);

const flightControlsCustom = walk("flight-controls-custom", [
  { press: inside(list, button("Tune the flight controls")), say: "A Job with work in flight" },
  { look: inside(controls, role("meter", "Context used")), say: "Compact carries a rule for the context window, 84% full" },
  { look: inside(controls, role("img", "2 subagents running")), say: "Subagents shows how many are running" },
  { look: tile("Checks"), say: "Checks pulses while any of the Job's Checks runs" },
  { press: tile("Auto land"), say: "Auto land lights while it is on" },
  { press: tile("Pause"), say: "Pause lights while the Job is paused, and reads Resume" },
  { press: tile("Resume"), say: "Resumed" },
  { press: tile("Edit"), say: "Edit turns the tiles into things to arrange, where they are" },
  { key: "Alt+ArrowRight", on: option("Ask for update"), say: "Alt and an arrow moves the tile under focus. Space picks one up and the arrows move it, Space puts it down" },
  { key: "Delete", on: option("Judge"), say: "Delete hides a tile, drawn quiet and dashed. Delete again brings it back" },
  { press: option("Add action"), say: "The + tile adds an action" },
  { type: "Hand off", into: inside(sheet("Add action"), role("textbox", "Name")), say: "A name, a glyph picked by its lucide name, and a key if it should have one" },
  { type: "Write down what is left, then stop.", into: inside(sheet("Add action"), role("textbox", "Words")), say: "Words are sent to the Job as Update's question is" },
  { press: inside(sheet("Add action"), button("Save")), say: "Saved into the tiles" },
  { press: option("Add action"), say: "A second action" },
  { type: "shell", into: inside(sheet("Add action"), role("combobox", "Kind")), say: "The kinds are Words, Slash command, Shell command and Page" },
  { type: "Typecheck", into: inside(sheet("Add action"), role("textbox", "Name")), say: "A name" },
  { type: "pnpm typecheck", into: inside(sheet("Add action"), role("textbox", "Shell line")), say: "A shell line runs in the Job's bay" },
  { press: inside(sheet("Add action"), button("Save")), say: "Saved" },
  { press: option("Compact job context"), say: "A built-in opens the same sheet" },
  { type: "/compact keep the plan", into: inside(sheet("Change Compact job context"), role("textbox", "Command")), say: "Compact's command, as it is typed to the Job. Reset puts Armada's own back" },
  { press: inside(sheet("Change Compact job context"), button("Save")), say: "Saved" },
  { press: tile("Done"), say: "Done keeps the tiles. Esc would have left them as they were" },
  { look: tile("Hand off"), say: "The new tiles take their place beside the built-in ones, and the hidden one is gone" },
  { press: tile("Typecheck"), say: "A Shell action runs in the Job's bay" },
  { look: region("Shell output"), say: "What it printed stands where the tiles were" },
  { press: inside(region("Shell output"), button("Close")), say: "Close brings the tiles back" },
  { key: "Escape", on: region(/^Job: Tune the flight controls/), say: "With no Job open the controls are the deck's own four" },
  { press: tile("Edit"), say: "Edit works the same here" },
  { press: option("Add action"), say: "Only a page can be added without a Job: shell needs a bay, words and commands need a Job" },
  { type: "Docs", into: inside(sheet("Add action"), role("textbox", "Name")), say: "A name" },
  { type: "https://docs.example/armada", into: inside(sheet("Add action"), role("textbox", "Address")), say: "In a Job's tiles, {branch}, {pr} and {repository} are filled from the Job" },
  { press: inside(sheet("Add action"), button("Save")), say: "Saved" },
  { press: tile("Done"), say: "Done" },
  { look: tile("Docs"), say: "The page opens from the tile" },
]);

export { flightControlsCustom as "flight-controls-custom" };
