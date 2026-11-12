// Settings › Job starts: one row per project, Start at once or Ask me, for the Jobs another Job or the Operator starts. His own
// starts always run at once. Over `job-starts-settings`.

import { place, role, systemPage, walk } from "../walk";

const jobStartsSettings = walk("job-starts-settings", [
  {
    press: place("System"),
    say: "System",
  },
  {
    press: systemPage("Job starts"),
    say: "Job starts, under Fleet",
  },
  {
    look: role("region", "Jobs started for you"),
    say: "A row per project, for Jobs another Job or the Operator starts",
  },
  {
    hover: role("img", "Ask me"),
    say: "Cockpit trust is on Ask me: those Jobs wait on their start card",
  },
  {
    select: "ask_me",
    in: role("combobox", "Jobs started for you in Pocket launch"),
    say: "Pocket launch to Ask me",
  },
  {
    look: role("combobox", "Jobs started for you in Pocket launch"),
    say: "Saved to the project at once",
  },
]);

export { jobStartsSettings as "job-starts-settings" };
