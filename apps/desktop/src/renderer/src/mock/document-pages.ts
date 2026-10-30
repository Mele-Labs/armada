// The document the `job-story` investigation wrote, as a Job writes one: a free HTML page of its own design that links the kit and
// embeds a live sketch. Same facts as `LEASE_REPORT` (files-disk.ts). Revision 2 is what the Job writes after the walk's notes: it
// answers each of them in the page. Mock only; a real Job writes the file with its own tools.

import { arrow, box, scene } from "./scenes";

export const LEASE_DOC_TITLE = "Why do slot leases expire early";
export const LEASE_DOC_PATH = "docs/reports/slot-lease-expiry.html";
export const LEASE_DOC_PATH_2 = "docs/reports/slot-lease-expiry.r2.html";

const SKETCH_TITLE = "Lease clock against a Check run";

const CLOCK = scene(
  [
    box("handed", 0, 0, "Bay handed over", "The Job gets the bay", { icon: "hard-drive", step: 1 }),
    box("clock", 330, 0, "Lease clock starts", "30 minutes", { icon: "clock", step: 2 }),
    { ...box("expires", 660, 0, "Lease expires", "The bay is free to lease again", { icon: "lock", step: 4 }), color: "red" as const },
    box("check", 330, 170, "Check runs", "The Drone waits", { icon: "shield-check", step: 3 }),
    { ...box("wakes", 660, 170, "Drone wakes", "Its lease is gone", { icon: "bot", step: 5 }), color: "amber" as const },
  ],
  [
    arrow("handed", "clock", undefined, undefined, 1),
    arrow("clock", "expires", "30 minutes", undefined, 4),
    arrow("clock", "check", "Drone waits", undefined, 3),
    arrow("check", "wakes", undefined, undefined, 5),
    arrow("expires", "wakes", "lease gone", undefined, 5),
  ],
);

const SKETCH = `<armada-sketch title="${SKETCH_TITLE}"><script type="application/json">${JSON.stringify(CLOCK)}</script></armada-sketch>`;

const ROWS: readonly (readonly [job: string, start: string, expired: string, check: string])[] = [
  ["Retry the slot lease", "Mon 09:12", "09:42", "Yes"],
  ["Pin the store clock", "Mon 13:05", "13:35", "Yes"],
  ["Cap the log reader", "Tue 10:20", "10:50", "Yes"],
  ["Rename the bay tile", "Tue 15:41", "16:11", "No"],
  ["Trim the pool log", "Wed 08:58", "09:28", "Yes"],
  ["Show the attempt", "Wed 14:30", "15:00", "Yes"],
  ["Move the lease sweep", "Thu 11:47", "12:17", "No"],
];

const NO_CAUSE = new Set(["Rename the bay tile", "Move the lease sweep"]);

const table = (second: boolean) => `<table>
  <caption>Early expiries</caption>
  <thead><tr><th>Job</th><th>Lease start</th><th>Expired</th><th>Check running</th></tr></thead>
  <tbody>
${ROWS.map(([job, start, expired, check]) => `    <tr><td>${job}</td><td class="when">${start}</td><td class="when">${expired}</td><td class="when">${second && NO_CAUSE.has(job) ? "No, cause unknown" : check}</td></tr>`).join("\n")}
  </tbody>
</table>`;

/** The page's own design: a reading column, a kicker over the title, a larger first paragraph and tabular figures. */
const STYLE = `
  body { max-inline-size: 46rem; }
  .kicker { margin: 0 0 var(--space-2); color: var(--fg-subtle); font-family: var(--font-mono); font-size: var(--text-2xs); line-height: var(--leading-2xs); letter-spacing: var(--tracking-caps); text-transform: uppercase; }
  .lede { font-size: var(--text-lg); line-height: var(--leading-lg); }
  h2 { padding-block-start: var(--space-3); border-block-start: var(--border-width) solid var(--border-subtle); }
  .when { font-family: var(--font-mono); font-variant-numeric: tabular-nums; white-space: nowrap; }
  tbody tr:focus-visible { outline: var(--edge-active) solid var(--edge-focus); outline-offset: calc(var(--edge-active) * -1); }
`;

type Revision = {
  lede: string;
  clockNote: string;
  measured: string;
  afterTable: string;
  notMeasured: string;
};

const page = (r: Revision, second: boolean) => `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>${LEASE_DOC_TITLE}</title>
<link rel="stylesheet" href="armada-kit.css">
<style>${STYLE}</style>
</head>
<body>
<header>
  <p class="kicker">Investigation</p>
  <h1>${LEASE_DOC_TITLE}</h1>
  <p class="lede">${r.lede}</p>
</header>

<section>
  <h2>How the lease clock runs</h2>
  <p>The clock starts at the handover. A Check the Drone starts keeps it waiting, and nothing it does while it waits counts as the Job writing.</p>
  ${SKETCH}
${r.clockNote}
</section>

<section>
  <h2>What was measured</h2>
  <p>Lease starts and expiries for 41 Jobs over four days, read from the pool log.</p>
  <p>${r.measured}</p>
  ${table(second)}
${r.afterTable}
</section>

<section>
  <h2>What was not measured</h2>
  <p>${r.notMeasured}</p>
  <ul>
    <li>Leases on a machine running more than four Jobs</li>
    <li>Jobs whose Checks run in a container
      <ul><li>A container Check that pulls its image first</li></ul>
    </li>
  </ul>
  <p>The next run, in order:</p>
  <ol>
    <li>Log each Check's start and end in the pool log</li>
    <li>Repeat the four days with the clock paused while a Check runs</li>
    <li>Compare the expiries</li>
  </ol>
</section>
</body>
</html>
`;

const FIRST: Revision = {
  lede: "The pool starts the 30 minute lease clock when it hands a bay over, not when the Job first writes to it, so a Job waiting on a long Check spends its lease before its Drone runs again. Five of the seven early expiries in four days of the pool log had a Check running when the lease ran out. Renewing the lease on each Fleet event for that Job would close the gap and leave the limit as it is.",
  clockNote: "",
  measured: "Seven leases expired before the Job finished. Five of them had a Check running in the bay at the time. The other two show no cause in the log.",
  afterTable: "",
  notMeasured: "Whether a renewal on every event adds load to the pool. The log holds no figure for it.",
};

const SECOND: Revision = {
  lede: "The pool starts the 30 minute lease clock when it hands a bay over, not when the Job first writes to it, so a Job waiting on a long Check spends its lease before its Drone runs again. Five of the seven early expiries in four days of the pool log had a Check running when the lease ran out. Renewing the lease on each Fleet event for that Job would close the gap and keep the 30 minute limit. The limit stays because it is what frees a bay when a Drone dies mid turn.",
  clockNote: "  <p>The clock does not stop while a Check runs. Any Check that outlasts 30 minutes ends the lease before the Drone wakes.</p>",
  measured: "Seven leases expired before the Job finished. Five of them had a Check running in the bay at the time. The other two had none, and the log shows no cause for either. They are marked in the table.",
  afterTable: `  <h3>Two with no cause</h3>
  <p>Rename the bay tile and Move the lease sweep. No Check ran in either bay, and the pool log records no other event before the lease ended.</p>`,
  notMeasured: "Whether a renewal on every event adds load to the pool. The log holds no figure for it. The cause of the two expiries with no Check running is not in the log either.",
};

/** The document as the Job first wrote it. */
export const LEASE_DOC_1 = page(FIRST, false);

/** The document after the notes, which the walk sends. */
export const LEASE_DOC_2 = page(SECOND, true);

// ---- The video script a Job writes for a product demo (`document-waits-in-needs-you`) ------------------------------------

export const VIDEO_SCRIPT_TITLE = "Video script";
/** Where the Job writes the page, and where Fleet keeps each revision of it. */
export const VIDEO_SCRIPT_PATH = "/Users/user/armada/.armada/slots/slot-2/.armada/documents/video-script.html";
export const VIDEO_SCRIPT_R1 = "documents/video-script/r1.html";
export const VIDEO_SCRIPT_R2 = "documents/video-script/r2.html";

const SCENES: readonly (readonly [time: string, shot: string, voice: string])[] = [
  ["0:00", "The Board, three Jobs running", "Three Jobs run while you read the first one's plan."],
  ["0:12", "A Job's thread, a plan card", "Each Job says what it will do before it starts."],
  ["0:25", "A walk window over the app", "When it has something to show, it opens a walk."],
  ["0:40", "The merge queue, one pull request landing", "What you approve lands on main."],
];

const script = (closing: string, second: boolean) => `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>${VIDEO_SCRIPT_TITLE}</title>
<link rel="stylesheet" href="armada-kit.css">
<style>
  body { max-inline-size: 46rem; }
  .when { font-family: var(--font-mono); font-variant-numeric: tabular-nums; white-space: nowrap; }
</style>
</head>
<body>
<header>
  <h1>${VIDEO_SCRIPT_TITLE}</h1>
  <p>A ${second ? "50" : "55"} second demo of Armada for the landing page: four shots and a closing line, read over a screen recording.</p>
</header>

<section>
  <h2>Shots</h2>
  <table>
    <thead><tr><th>At</th><th>Shot</th><th>Voice</th></tr></thead>
    <tbody>
${SCENES.map(([time, shot, voice]) => `      <tr><td class="when">${time}</td><td>${shot}</td><td>${voice}</td></tr>`).join("\n")}
    </tbody>
  </table>
</section>

<section>
  <h2>Closing</h2>
  <p>${closing}</p>
</section>

<section>
  <h2>Recording</h2>
  <ul>
    <li>The mock app, dark theme, at 1440 by 900</li>
    <li>No cursor trails, no zoom</li>
  </ul>
</section>
</body>
</html>
`;

/** The script as the Job first wrote it. */
export const VIDEO_SCRIPT_1 = script("Armada runs your agents and checks their work. Try it on your own repository.", false);

/** The script's second revision, with a shorter close. */
export const VIDEO_SCRIPT_2 = script("Your agents, checked. Try Armada.", true);
