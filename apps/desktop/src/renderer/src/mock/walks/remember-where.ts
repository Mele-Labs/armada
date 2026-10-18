// Where a repository's runs are kept. Dispatch for my-app, Remember asking once with the four places, iCloud Drive picked, the rows marked with its glyph,
// and a later Dispatch whose Remember keeps without asking. Over the `remember-where` scenario.

import { button, inside, region, role, walk } from "../walk";

const jobs = region("Jobs", { exact: true });
const dispatch = role("region", "Dispatch", { exact: true });
const runs = inside(dispatch, role("group", "How my-app runs"));
const places = inside(runs, role("group", "Where to remember"));
const remember = inside(dispatch, button("Remember", { exact: true }));

const rememberWhere = walk("remember-where", [
  { key: "n", on: jobs, say: "N opens Dispatch for my-app. Its runs were seen, not kept" },
  { press: remember, say: "Remember asks where. It asks once for my-app" },
  { press: inside(places, button(/^iCloud or Google Drive/)), say: "A synced folder lists the ones found" },
  { press: inside(places, button(/^iCloud Drive/)), say: "iCloud Drive is picked" },
  { later: inside(runs, role("img", "Tests: Remembered in iCloud Drive")), say: "Each row wears the folder glyph. A later Job ran pnpm lint" },
  { key: "Escape", on: dispatch, say: "Esc closes Dispatch" },
  { key: "n", on: jobs, say: "N opens the next Dispatch for my-app. Lint is seen, not kept" },
  { press: remember, say: "Remember keeps it in iCloud Drive without asking" },
  { look: inside(runs, role("img", "Lint: Remembered in iCloud Drive")), say: "Lint wears the same glyph" },
]);

export { rememberWhere as "remember-where" };
