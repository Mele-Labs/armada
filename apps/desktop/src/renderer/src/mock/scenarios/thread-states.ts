// A Job in each state a thread can be in, for the sheet `?sheet=thread-states` (`thread-sheets.tsx`), which draws each one
// today and in the spacious take side by side: a long message, a run of calls, code, a table, a failed run, a walk, an
// empty thread, and the document waiting in Needs you. The working, reasoning and done Jobs are the walk's own
// (`spacious-thread.ts`).

import type {
  ArtifactRead,
  Session,
  SessionRow,
  VideoAddress,
} from "@armada/screens/src/draft/sessions";

import landscape from "../media/landscape.jpg";
import portrait from "../media/portrait.jpg";
import recording from "../media/recording.mp4";

import { writeKept } from "@armada/components";

import type { Scenario } from "../moment";
import { videoScriptScenario } from "../video-script-job";
import {
  spaciousCallingJob,
  spaciousFinishedJob,
  spaciousReasonedJob,
  spaciousThinkingJob,
} from "./spacious-thread";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const you = (id: string, at: string, text: string): SessionRow => ({
  id,
  at,
  kind: "message",
  from: {
    kind: "you",
  },
  text,
});

const agent = (id: string, at: string, text: string): SessionRow => ({
  id,
  at,
  kind: "message",
  from: {
    kind: "agent",
  },
  text,
});

const job = (id: string, title: string, rows: readonly SessionRow[]): Session => ({
  id,
  title,
  turn: {
    state: "idle",
  },
  lastTurnAt: ago(3),
  rows,
  attachments: [],
});

const LONG_ASK =
  "The onboarding flow loses people at the repository step. Most of them have a repository open in another window already, but the picker only lists folders under the home directory, so anyone who keeps code on a second drive or under a work folder has to type the path by hand. Half the people who reach that step stop there. I want the picker to list the repositories Armada has seen before, the ones open in any editor it can read, and anything under the folders they have chosen in the past, in that order, with the path typed by hand as the last way in rather than the first. Keep the step to one screen and do not add a second one for the folders.";

const LONG_ANSWER =
  "The picker reads one folder today: `onboarding/picker.ts` lists the home directory and nothing else, so a repository anywhere else is a typed path.\n\nThree sources would fill it in the order you gave. Armada already keeps the repositories a Job ran in, which covers anyone who has used it before. Editors that write a recent list, such as VS Code and Zed, can be read without asking for anything. The folders chosen in the past are in the settings store under `picker.roots`.\n\nAll three fit on the one screen as sections of the same list. The typed path stays as the last row. I have not changed anything yet; the list order and the editors to read are yours to say.";

/** A Job whose thread is one long message each way. */
const long = job("st1", "Onboarding picker", [
  you("st1a", "09:10:00", LONG_ASK),
  agent("st1b", "09:12:40", LONG_ANSWER),
]);

/** A Job whose agent made many calls in a row. */
const calls = job("st2", "Port scan for the dev server", [
  you("st2a", "09:20:00", "Find why the dev server picks a port that is already taken."),
  ...[
    "Read apps/desktop/src/main/dev-server.ts",
    "Read apps/desktop/src/main/ports.ts",
    "Grep findFreePort in apps",
    "Read apps/desktop/src/main/ports.test.ts",
    "Bash lsof -i :5173",
    "Read crates/fleet/src/serve/ports.rs",
    "Grep reserve_port in crates",
    "Read crates/fleet/src/serve/lease.rs",
    "Bash cargo tree -p fleet",
    "Read docs/practices/ports.md",
    "Edit apps/desktop/src/main/ports.ts",
    "Edit apps/desktop/src/main/ports.test.ts",
  ].map(
    (text, at): SessionRow => ({
      id: `st2c${at}`,
      at: `09:20:${String(10 + at * 3).padStart(2, "0")}`,
      kind: "tool",
      text,
    }),
  ),
  agent(
    "st2b",
    "09:21:30",
    "The port is checked and then taken a moment later, and Fleet's own lease can take it in between. It now holds the socket open until the server binds.",
  ),
]);

/** A Job whose agent answered with code. */
const code = job("st3", "Retry helper", [
  you("st3a", "09:30:00", "Show me the retry helper before you use it."),
  agent(
    "st3b",
    "09:30:50",
    "It retries a call up to `tries` times, waiting longer each time:\n\n```ts\nexport async function retry<T>(call: () => Promise<T>, tries = 3): Promise<T> {\n  for (let at = 1; ; at++) {\n    try {\n      return await call();\n    } catch (error) {\n      if (at === tries) throw error;\n      await new Promise((done) => setTimeout(done, 250 * 2 ** at));\n    }\n  }\n}\n```\n\nThe wait doubles from half a second, so three tries take at most three and a half seconds.",
  ),
]);

/** A Job whose agent answered with a table. */
const table = job("st4", "Plan limits", [
  you("st4a", "09:40:00", "Compare the three plans on what a Job can use."),
  agent(
    "st4b",
    "09:40:35",
    "| Plan | Jobs at once | Context | Model |\n| --- | --- | --- | --- |\n| Free | 1 | 200k | sonnet |\n| Pro | 3 | 400k | opus or sonnet |\n| Team | 10 | 1M | any |\n\nOnly Team lifts the context past 400k.",
  ),
]);

/** A Job whose run failed. */
const failed = job("st5", "Fix the flaky clock test", [
  you("st5a", "09:50:00", "Make the clock test pass on CI."),
  {
    id: "st5b",
    at: "09:51:12",
    kind: "ran",
    command: "pnpm exec vitest run src/clock.test.ts",
    code: 1,
    tail: [
      " FAIL  src/clock.test.ts > ticks once a second",
      " AssertionError: expected 2 to be 1",
      "  ❯ src/clock.test.ts:41:22",
      " Test Files  1 failed (1)",
    ].join("\n"),
  },
  agent(
    "st5c",
    "09:51:30",
    "It fails here too: the fake timer advances two ticks because the test starts it before the clock subscribes.",
  ),
]);

/** A Job that put a walk to him. */
const walked = job("st6", "Settings search", [
  you("st6a", "10:00:00", "Let me find a setting by typing its name."),
  agent(
    "st6b",
    "10:08:20",
    "Settings has a search box at the top. Typing filters the sections as you type.",
  ),
  {
    id: "st6c",
    at: "10:08:30",
    kind: "window",
    title: "Settings search",
    url: "http://localhost:40024/?walk=settings-search",
    takes: 2,
  },
]);

/** A Job waiting on his answer in Needs you. */
const asks: Session = {
  ...job("st8", "Staging migration", [
    you("st8a", "10:20:00", "Get the store migration ready for staging."),
    agent(
      "st8b",
      "10:24:10",
      "The migration runs clean on a copy of production. It locks the runs table for about 40 seconds, so it needs a quiet moment on staging.",
    ),
  ]),
  waitingFor: [
    {
      id: "w-migrate",
      text: "Choose when the migration runs on staging",
      since: ago(2),
      source: "agent",
    },
  ],
};

const tool = (id: string, at: string, text: string): SessionRow => ({
  id,
  at,
  kind: "tool",
  text,
});

const SHOTS = "/Users/user/armada/.armada/slots/slot-2/.playwright-mcp";

/** A Job like the one this take was made in: started by another, short lines between its calls, and calls running. */
const ours: Session = {
  ...job("st9", "Spacious Job thread", [
    tool("st9a", "22:04:20", `Read ${SHOTS}/sheet-1.png`),
    agent(
      "st9b",
      "22:04:28",
      "Two problems: the sheet's background is see-through, so `--bg-canvas` must not exist, and focusing the paragraph scrolled the page to the bottom. Finding the page background token.",
    ),
    tool("st9c", "22:04:40", `Read ${SHOTS}/sheet-1.png`),
    agent(
      "st9d",
      "22:04:48",
      "The sheet draws both looks side by side. Scrolling through the rest to check each state.",
    ),
    tool("st9e", "22:04:50", "mcp__plugin_playwright_playwright__browser_evaluate"),
    agent(
      "st9f",
      "22:04:53",
      "All 14 rows draw. Checking the riskier ones: the failed run with Needs you, the ending, and the empty and narrow panes.",
    ),
    tool(
      "st9g",
      "22:05:00",
      'Bash grep -n "const waits =" apps/desktop/src/renderer/src/sessions.tsx',
    ),
    tool("st9h", "22:05:10", "Bash sed -n 2596,2620p apps/desktop/src/renderer/src/sessions.tsx"),
    agent(
      "st9i",
      "22:05:26",
      "Still tracing why the Needs you band is missing on the sheet. Checking whether it's the sheet's mount or the scenario, by opening the same Job in the plain app.",
    ),
    tool("st9j", "22:05:30", "mcp__plugin_playwright_playwright__browser_evaluate"),
    agent(
      "st9k",
      "22:05:36",
      "In the app the band shows, so the sheet's mount is missing something the Cockpit passes. Reading the Cockpit's SessionView mount.",
    ),
    tool(
      "st9l",
      "22:05:40",
      'Bash grep -n "<SessionView" -A40 apps/desktop/src/renderer/src/cockpit/deck/panes.tsx',
    ),
    tool("st9m", "22:05:44", "Grep inPanes in apps/desktop/src/renderer/src"),
  ]),
  turn: {
    state: "working",
  },
  attachments: [
    {
      kind: "started_by",
      id: "st-operator",
    },
  ],
};

/** The Job that started it, there for its line to name. */
const operator = job("st-operator", "Temporary Operator", []);

const PICTURES: Record<string, string> = {
  [`${SHOTS}/cockpit-wide.jpg`]: landscape,
  [`${SHOTS}/job-on-the-phone.jpg`]: portrait,
};
const VIDEO = `${SHOTS}/cockpit-recording.mp4`;
const DOC = "/Users/user/armada/docs/scope.md";

/** A Job that showed pictures, a video and a doc with `show_file`. */
const media: Session = {
  ...job("st10", "Cockpit on the phone", [
    you("st10a", "14:02:10", "Show me what the Cockpit looks like now."),
    agent("st10b", "14:06:00", "A short recording of it running."),
    tool("st10c", "14:06:04", `mcp__armada-fleet__show_file ${VIDEO}`),
    agent("st10d", "14:07:00", "The Cockpit on a wide window, and a Job on the phone."),
    tool("st10e", "14:07:04", `mcp__armada-fleet__show_file ${SHOTS}/cockpit-wide.jpg`),
    tool("st10f", "14:07:09", `mcp__armada-fleet__show_file ${SHOTS}/job-on-the-phone.jpg`),
    agent("st10g", "14:07:30", "And the scope doc the layout follows."),
    tool("st10h", "14:07:34", `mcp__armada-fleet__show_file ${DOC}`),
  ]),
  attachments: [
    ...Object.keys(PICTURES),
    VIDEO,
    DOC,
  ].map((path) => ({
    kind: "artifact" as const,
    form: "file" as const,
    id: path,
    title: path.slice(path.lastIndexOf("/") + 1),
  })),
};

/** A run of mixed calls between two messages, as this take's own thread had one: a commit, two pages shown, a task ticked. */
const chain = job("st11", "Show the sheets", [
  agent(
    "st11a",
    "22:08:25",
    "The walk still plays, and types pass. Committing, then showing both pages.",
  ),
  tool("st11b", "22:08:27", "Bash git add -A apps/desktop/src/renderer/src/mock"),
  {
    id: "st11c",
    at: "22:08:33",
    kind: "window",
    title: "Thread states, today and spacious",
    url: "http://localhost:40024/?scenario=thread-states&sheet=thread-states",
  },
  tool("st11d", "22:08:33", "mcp__armada-fleet__show_window"),
  {
    id: "st11e",
    at: "22:08:34",
    kind: "window",
    title: "Where the spacious take reaches",
    url: "http://localhost:40024/?scenario=thread-states&sheet=ripple",
  },
  tool("st11f", "22:08:35", "TaskUpdate"),
  agent("st11g", "22:08:43", "Both pages are open in the Job's tabs."),
]);

/** A Job that leased a slot for its branch, worked in it, and gave it back. */
const slotted = job("st12", "Kept builds", [
  you("st12a", "10:30:00", "Keep the last two builds so a bad one can be undone."),
  {
    id: "st12b",
    at: "10:30:20",
    kind: "lease",
    slot: 3,
    branch: "armada/kept-builds",
  },
  tool("st12c", "10:31:02", "Edit crates/fleet/src/builds.rs"),
  agent(
    "st12d",
    "10:39:18",
    "Step 1 landed. The last two builds are kept under Armada/builds, and Revert puts the previous one back.",
  ),
  {
    id: "st12e",
    at: "10:39:25",
    kind: "release",
    slot: 3,
    branch: "armada/kept-builds",
  },
]);

/** A Job whose answer is one long block of code, longer than the pane. */
const longCode = job("st13", "Revert to the last build", [
  you("st13a", "11:02:00", "Write the module that keeps builds and puts the previous one back."),
  agent(
    "st13b",
    "11:06:40",
    'Here it is, in `builds.ts`:\n\n```ts\nimport { readFile } from "node:fs/promises";\n\nexport type Build = {\n  id: string;\n  at: number;\n  path: string;\n};\n\n/** The builds kept on disk, newest first, read from the folder\'s index. */\nexport async function keptBuilds(folder: string): Promise<Build[]> {\n  const text = await readFile(`${folder}/index.json`, "utf8");\n  const builds = JSON.parse(text) as Build[];\n  return builds.sort((a, b) => b.at - a.at);\n}\n\n/** The build to go back to: the newest that is not the one running. */\nexport function previous(builds: readonly Build[], running: string): Build | null {\n  for (const one of builds) {\n    if (one.id !== running) return one;\n  }\n  return null;\n}\n\n/** Keeps the newest `keep` builds and names the rest to delete. */\nexport function pruned(builds: readonly Build[], keep = 2): {\n  kept: Build[];\n  gone: Build[];\n} {\n  const sorted = [...builds].sort((a, b) => b.at - a.at);\n  return {\n    kept: sorted.slice(0, keep),\n    gone: sorted.slice(keep),\n  };\n}\n\n/** Puts the previous build back, and says which it was. */\nexport async function revert(folder: string, running: string): Promise<Build> {\n  const builds = await keptBuilds(folder);\n  const back = previous(builds, running);\n  if (back === null) throw new Error("No earlier build is kept");\n  await swap(folder, back);\n  return back;\n}\n\nasync function swap(folder: string, to: Build): Promise<void> {\n  const { rename, symlink, unlink } = await import("node:fs/promises");\n  const link = `${folder}/current`;\n  const next = `${folder}/current.next`;\n  await symlink(to.path, next);\n  await rename(next, link).catch(async () => {\n    await unlink(link);\n    await rename(next, link);\n  });\n}\n```\n\nThe swap is a rename over a link, so a running Fleet never sees a half-written one.',
  ),
]);

/** A Job with two pictures, a file and a sketch in its box, waiting to go with the next message. */
const attaching = job("st14", "Phone layout", [
  you("st14a", "15:20:00", "The Job page on the phone cuts the composer off."),
  agent(
    "st14b",
    "15:24:10",
    "Send me a screenshot of where it cuts off and I will match the breakpoint to it.",
  ),
]);
writeKept(
  "session:st14",
  {
    text: "",
    tags: [],
    files: [
      {
        id: "st14-wide",
        name: "cockpit-wide.jpg",
        src: landscape,
      },
      {
        id: "st14-phone",
        name: "job-on-the-phone.jpg",
        src: portrait,
      },
      {
        id: "st14-log",
        name: "renderer-console.log",
      },
    ],
  },
  {
    text: "",
    tags: [],
  },
);

/** A Job with nothing in its thread yet. */
const empty = job("st7", "New Job", []);

const base = videoScriptScenario(
  "thread-states",
  "A Job in each state a thread can be in, for the sheet that draws them today and in the spacious take",
  1,
  [
    long,
    calls,
    code,
    table,
    failed,
    walked,
    asks,
    ours,
    chain,
    slotted,
    longCode,
    attaching,
    operator,
    media,
    empty,
    spaciousThinkingJob,
    spaciousCallingJob,
    spaciousReasonedJob,
    spaciousFinishedJob,
  ],
);

// The pictures and the video are the phone fixture's, as `thread-media.ts` draws them; the rest reads as the script's Job does.
export const threadStates: Scenario = {
  ...base,
  draft: {
    ...base.draft,
    sessions: (board, more) => {
      const store = base.draft!.sessions!(board, more);
      return {
        ...store,
        readArtifact: async (id: string, path: string): Promise<ArtifactRead> => {
          const url = PICTURES[path];
          if (url === undefined)
            return (
              store.readArtifact?.(id, path) ?? {
                ok: false,
                why: "unreadable",
              }
            );
          return {
            ok: true,
            bytes: new Uint8Array(await (await fetch(url)).arrayBuffer()),
            type: "image/jpeg",
          };
        },
        videoAddress: async (_id: string, path: string): Promise<VideoAddress> =>
          path === VIDEO
            ? {
                ok: true,
                address: recording,
              }
            : {
                ok: false,
                says: "This video cannot play here.",
              },
        openFile: () => undefined,
      };
    },
  },
};
