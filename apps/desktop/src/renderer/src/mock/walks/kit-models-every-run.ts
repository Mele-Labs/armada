// Kit › Models: a select for every run that names a model or an effort. Over `system-sidebar`.

import { region, role, toSystemPage, walk } from "../walk";

export const kitModelsEveryRun = walk("system-sidebar", [
  ...toSystemPage("Models", "Models"),
  { look: region("Models"), say: "Judge, Proposer, Retro, Second opinion, Step down to, Helm effort" },
  { type: "fable", into: role("combobox", "Judge", { exact: true }), say: "Judge changed" },
]);
