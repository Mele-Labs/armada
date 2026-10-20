// Kit › Models: a select for every run that names a model or an effort. Over `system-sidebar`.

import { region, role, toSystemPage, walk } from "../walk";

export const kitModelsEveryRun = walk("system-sidebar", [
  ...toSystemPage("Models", "Models"),
  { look: region("Default models"), say: "Each run: its model, then its effort" },
  { type: "fable", into: role("combobox", "Judge", { exact: true }), say: "Judge changed" },
  { look: role("slider", "Judge effort"), say: "Drag the effort, or ← → a step at a time" },
]);
