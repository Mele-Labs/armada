// Start runs his Job at once; Draft is the one way his own work waits on its start card; the card under the sheet starts a quick
// Job as ⌘T does, opened on its empty thread. Over `job-start-flow`.

import { inside, region, role, walk } from "../walk";

const jobs = region("Jobs", {
  exact: true,
});
const frame = role("region", "Dispatch", {
  exact: true,
});
const request = inside(frame, role("textbox", "Request"));
const armada = inside(
  frame,
  role("radio", "Repository armada", {
    exact: true,
  }),
);
const sheetButton = (name: string) =>
  inside(
    frame,
    role("button", name, {
      exact: true,
    }),
  );
const quick = role("button", "New Job in armada");
const message = role("textbox", "Message");

const jobStartFlow = walk("job-start-flow", [
  {
    key: "Meta+n",
    on: jobs,
    say: "⌘N opens Dispatch",
  },
  {
    type: "Add a retry cap",
    into: request,
    say: "Words for a Job",
  },
  {
    press: armada,
    say: "The repository it runs in",
  },
  {
    hover: sheetButton("Draft"),
    say: "Draft sits beside Start, on ⌥↵",
  },
  {
    press: sheetButton("Start"),
    say: "Start runs the Job at once",
  },
  {
    look: message,
    say: "It opens working on your words, with no start card",
  },
  {
    key: "Meta+n",
    on: message,
    say: "⌘N opens Dispatch again",
  },
  {
    hover: quick,
    say: "Under the sheet, a card starts a quick Job, as ⌘T does",
  },
  {
    key: "Enter",
    on: quick,
    say: "Tab reaches it, and ↵ starts it",
  },
  {
    look: message,
    say: "The quick Job opens on its empty thread, the composer focused",
  },
  {
    key: "Meta+n",
    on: message,
    say: "Dispatch once more",
  },
  {
    type: "Rename settings",
    into: request,
    say: "Words to hold",
  },
  {
    press: armada,
    say: "The repository",
  },
  {
    press: sheetButton("Draft"),
    say: "Draft",
  },
  {
    look: role("button", "Start", {
      exact: true,
    }),
    say: "It opens on its start card, and nothing runs until Start",
  },
]);

export { jobStartFlow as "job-start-flow" };
