// One Job across about two hours, for the Artifacts graph. The request names two links and an issue; a sketch is drawn from it; the plan is
// recorded with three groups; the first shows a page twice, writes a file and a picture; the second opens a pull request that closes the issue
// and goes opened, red, green, merged, while the agent writes three links; the third starts a child Job. A later note of the owner's leads to a
// redraw of the sketch and the page shown again. `versions` is what Fleet owes (`Session.versions`), each stamped with `from`. Mock only.
// The walk `artifact-graph` plays it.

import type { Scene } from "@armada/components";
import type { Session, SessionAttachment, SessionRow } from "@armada/screens/src/draft/sessions";

import { READS, READS_AGAIN, READS_TITLE } from "./sketches";

export const GRAPH_JOB = "graph-1";
export const GRAPH_CHILD = "graph-2";
export const GRAPH_TITLE = "Trace the store clock";
export const GRAPH_SKETCH = READS_TITLE;
export const GRAPH_NOTE = "Can the report show the drift?";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

type Link = Extract<SessionAttachment, { kind: "link" }>;
type Sketch = Extract<SessionAttachment, { kind: "sketch" }>;

const link = (url: string, by: Link["by"]): Link => ({ kind: "link", url, by });

const LINKS = {
  rfc: link("https://example.com/rfc/injectable-clocks", "you"),
  second: link("https://example.com/notes/flaky-second-boundary", "you"),
  instant: link("https://example.com/docs/instant-vs-system-time", "agent"),
  fake: link("https://example.com/guides/fake-clock-in-tests", "agent"),
  changelog: link("https://example.com/changelog/store-0-14", "agent"),
};

const issue = (state: "open" | "closed"): SessionAttachment => ({ kind: "issue", number: "1887", title: "store_flaky fails when the second turns", address: "https://git.example/armada/issues/1887", state });

const pull = (checks: Extract<SessionAttachment, { kind: "pull_request" }>["checks"], state: "open" | "merged"): SessionAttachment => ({
  kind: "pull_request",
  number: 1913,
  title: "Pin the clock in the store tests",
  branch: "armada/pin-store-clock",
  address: "https://git.example/armada/pull/1913",
  checks,
  state,
  auto: false,
  closes: [1887],
});

const sketch = (drawing: Scene): Sketch => ({ kind: "sketch", id: `sketch:${READS_TITLE}`, title: READS_TITLE, by: "agent", drawing });

const WINDOW = "https://example.com/preview/clock-report";
const FILE = "/Users/user/armada/docs/spikes/store-clock.md";
const IMAGE = "/tmp/store-clock-failure.png";
const page = (): SessionAttachment => ({ kind: "artifact", form: "window", id: WINDOW, title: "Clock report" });
const file: SessionAttachment = { kind: "artifact", form: "file", id: FILE, title: "store-clock.md", by: "agent", written: true };
const picture: SessionAttachment = { kind: "artifact", form: "image", id: IMAGE, title: "store-clock-failure.png" };

const you = (id: string, minutes: number, text: string): SessionRow => ({ id, at: ago(minutes), kind: "message", from: { kind: "you" }, text });
const agent = (id: string, minutes: number, text: string): SessionRow => ({ id, at: ago(minutes), kind: "message", from: { kind: "agent" }, text });
const tool = (id: string, minutes: number, text: string): SessionRow => ({ id, at: ago(minutes), kind: "tool", text });
const shown = (id: string, minutes: number): SessionRow => ({ id, at: ago(minutes), kind: "window", title: "Clock report", url: WINDOW });

/** Every version of every attachment, oldest first, each at the minute it stood from and stamped with what it followed. */
function versions(): NonNullable<Session["versions"]> {
  const at = (minutes: number, attachment: SessionAttachment, from?: { message?: string; task?: string }) => ({ at: ago(minutes), attachment, ...(from === undefined ? {} : { from }) });
  const request = { message: "r1" };
  return [
    at(124, LINKS.rfc, request),
    at(124, LINKS.second, request),
    at(123, issue("open"), request),
    at(117, sketch(READS), request),
    at(100, page(), { task: "t2" }),
    at(96, file, { task: "t1" }),
    at(92, picture, { task: "t1" }),
    at(88, page(), { task: "t2" }),
    at(74, LINKS.instant, { task: "t3" }),
    at(70, pull({ state: "pending" }, "open"), { task: "t4" }),
    at(66, LINKS.fake, { task: "t3" }),
    at(62, pull({ state: "failed", failing: "store_flaky" }, "open"), { task: "t4" }),
    at(58, LINKS.changelog, { task: "t3" }),
    at(55, pull({ state: "passed" }, "open"), { task: "t4" }),
    at(50, { kind: "started_to", id: GRAPH_CHILD, group: "g3" }, { task: "t5" }),
    at(45, pull({ state: "passed" }, "merged"), { task: "t4" }),
    at(44, issue("closed")),
    at(36, sketch(READS_AGAIN), { message: "r2" }),
    at(30, page(), { message: "r2" }),
  ];
}

export function graphJob(): Session {
  return {
    id: GRAPH_JOB,
    title: GRAPH_TITLE,
    turn: { state: "idle" },
    lastTurnAt: ago(10),
    rows: [
      you("r1", 125, `Read ${LINKS.rfc.url} and ${LINKS.second.url}, then tell me whether the store can take its clock from outside. The failing test is #1887.`),
      tool("a1", 117, `show_sketch ${JSON.stringify({ title: READS_TITLE, scene: READS })}`),
      agent("a2", 112, "The plan is in three groups."),
      shown("a3", 100),
      shown("a4", 88),
      agent("a5", 74, `The difference between the two clocks is in ${LINKS.instant.url}.`),
      agent("a6", 66, `A fake clock is the usual answer, ${LINKS.fake.url}`),
      agent("a7", 58, `The store's last release changed the clock: ${LINKS.changelog.url}`),
      agent("a8", 45, "The pull request merged and closed #1887."),
      you("r2", 40, GRAPH_NOTE),
      tool("a9", 36, `show_sketch ${JSON.stringify({ title: READS_TITLE, scene: READS_AGAIN })}`),
      shown("a10", 30),
    ],
    attachments: [
      LINKS.rfc,
      LINKS.second,
      issue("closed"),
      sketch(READS_AGAIN),
      page(),
      file,
      picture,
      LINKS.instant,
      LINKS.fake,
      LINKS.changelog,
      { kind: "started_to", id: GRAPH_CHILD, group: "g3" },
      pull({ state: "passed" }, "merged"),
    ],
    versions: versions(),
    work: {
      railOff: true,
      plan: {
        version: 1,
        scope: ["crates/store"],
        approved: true,
        after: "a2",
        groups: [
          {
            id: "g1",
            agent: "rust-engineer",
            title: "Inject the clock",
            paths: ["crates/store"],
            tasks: [
              { id: "t1", text: "Take the clock as a parameter", state: "done" },
              { id: "t2", text: "Show the report page", state: "done" },
            ],
          },
          {
            id: "g2",
            agent: "rust-engineer",
            title: "Pin it in the tests",
            paths: ["crates/store"],
            tasks: [
              { id: "t3", text: "Fake the clock in the store tests", state: "done" },
              { id: "t4", text: "Open the pull request", state: "done" },
            ],
          },
          { id: "g3", agent: "rust-engineer", title: "Check the report", paths: ["crates/store"], tasks: [{ id: "t5", text: "Run the report on the pinned clock", state: "done" }] },
        ],
      },
    },
  };
}

export function graphChild(): Session {
  return {
    id: GRAPH_CHILD,
    title: "Run the report on the pinned clock",
    turn: { state: "idle" },
    lastTurnAt: ago(30),
    rows: [agent("c1", 49, "Started on the group Check the report.")],
    attachments: [{ kind: "started_by", id: GRAPH_JOB, group: "g3" }],
  };
}
