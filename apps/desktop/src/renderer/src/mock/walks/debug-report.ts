// Copy debug report: a control on Doctor, a sheet with the text it will copy, and Copy. The second opening is the
// report with Fleet not answering. Over the `debug-report` scenario.

import { button, dialog, inside, role, viaPalette, walk } from "../walk";

const sheet = dialog("Debug report");

const debugReport = walk("debug-report", [
  ...viaPalette("Doctor", "Doctor, with Copy debug report in its header"),
  { press: button("Copy debug report"), say: "The report as it will be copied: install, mod, repository, each Job, recent trouble" },
  { press: inside(sheet, button("Copy", { exact: true })), say: "Copy, and its icon turns to a check" },
  { key: "Escape", on: inside(sheet, button("Close")), say: "Esc closes the sheet, and focus is back on Copy debug report" },
  { press: button("Copy debug report"), say: "With Fleet not answering: the install part and the log, and a line saying so" },
  { key: "Escape", on: inside(sheet, button("Close")), say: "Close it" },
  { key: "Meta+k", on: role("navigation", "Places"), say: "⌘K, for the same report from the palette" },
  { type: "debug", into: inside(dialog("Command palette"), role("combobox")), say: "Debug Bridge stays first, so Return still opens it. Copy debug report is the row under it" },
]);

export { debugReport as "debug-report" };
