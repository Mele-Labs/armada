// One Job across about two hours, for the Artifacts timeline: links written by the person and by the agent, two issues, a pull request that
// goes through failed and passed Checks to merged and closes the first issue, a sketch drawn three times, a page and a file each shown
// twice, a picture, and a child Job started from a plan group. `versions` is what Fleet owes (`Session.versions`); the attachments are the
// ends of it. Mock only. The walk `artifact-timeline` plays it.

import type { Scene } from "@armada/components";
import type { Session, SessionAttachment, SessionRow } from "@armada/screens/src/draft/sessions";

import { PINNED, READS, READS_AGAIN, READS_TITLE } from "./sketches";

export const TIMELINE_JOB = "timeline-1";
export const TIMELINE_CHILD = "timeline-2";
export const TIMELINE_TITLE = "Trace the store clock";
export const TIMELINE_SKETCH = READS_TITLE;

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const clock = (minutes: number) => new Date(Date.now() - minutes * 60_000).toTimeString().slice(0, 8);

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

const issue = (number: string, title: string, state: "open" | "closed"): SessionAttachment => ({ kind: "issue", number, title, address: `https://git.example/armada/issues/${number}`, state });

const pull = (number: number, checks: Extract<SessionAttachment, { kind: "pull_request" }>["checks"], state: "open" | "merged"): SessionAttachment => ({
  kind: "pull_request",
  number,
  title: "Pin the clock in the store tests",
  branch: "armada/pin-store-clock",
  address: `https://git.example/armada/pull/${number}`,
  checks,
  state,
  auto: false,
  closes: [1887],
});

const sketch = (drawing: Scene, against?: { title: string; drawing: Scene }): Sketch => ({
  kind: "sketch",
  id: `sketch:${READS_TITLE}`,
  title: READS_TITLE,
  by: "agent",
  drawing,
  ...(against === undefined ? {} : { against }),
});

const WINDOW = "https://example.com/preview/clock-report";
const FILE = "/Users/user/armada/docs/spikes/store-clock.md";
const IMAGE = "/tmp/store-clock-failure.png";
const page = (): SessionAttachment => ({ kind: "artifact", form: "window", id: WINDOW, title: "Clock report" });
const file = (minutes: number): SessionAttachment => ({ kind: "artifact", form: "file", id: FILE, title: "store-clock.md", by: "agent", shown: ago(minutes) });
const picture: SessionAttachment = { kind: "artifact", form: "image", id: IMAGE, title: "store-clock-failure.png" };

const you = (id: string, minutes: number, text: string): SessionRow => ({ id, at: clock(minutes), kind: "message", from: { kind: "you" }, text });
const agent = (id: string, minutes: number, text: string): SessionRow => ({ id, at: clock(minutes), kind: "message", from: { kind: "agent" }, text });
const tool = (id: string, minutes: number, text: string): SessionRow => ({ id, at: clock(minutes), kind: "tool", text });
const shown = (id: string, minutes: number): SessionRow => ({ id, at: clock(minutes), kind: "window", title: "Clock report", url: WINDOW });
const drew = (id: string, minutes: number, drawing: Scene, against?: string): SessionRow =>
  tool(id, minutes, `show_sketch ${JSON.stringify({ title: READS_TITLE, scene: drawing, ...(against === undefined ? {} : { against }) })}`);

/** Every version of every attachment, oldest first, each at the minute it stood from. */
function versions(): NonNullable<Session["versions"]> {
  const first = sketch(READS);
  const second = sketch(READS_AGAIN, { title: READS_TITLE, drawing: READS });
  const third = sketch(PINNED, { title: READS_TITLE, drawing: READS_AGAIN });
  const at = (minutes: number, attachment: SessionAttachment) => ({ at: ago(minutes), attachment });
  return [
    at(124, LINKS.rfc),
    at(120, issue("1887", "store_flaky fails when the second turns", "open")),
    at(115, issue("1890", "The store clock cannot be injected", "open")),
    at(108, LINKS.instant),
    at(100, first),
    at(97, LINKS.second),
    at(90, page()),
    at(81, LINKS.fake),
    at(78, second),
    at(70, file(70)),
    at(64, { kind: "started_to", id: TIMELINE_CHILD, group: "g2" }),
    at(60, picture),
    at(55, pull(1913, { state: "pending" }, "open")),
    at(52, third),
    at(48, pull(1913, { state: "failed", failing: "store_flaky" }, "open")),
    at(44, LINKS.changelog),
    at(40, page()),
    at(33, pull(1913, { state: "passed" }, "open")),
    at(25, file(25)),
    at(14, pull(1913, { state: "passed" }, "merged")),
    at(12, issue("1887", "store_flaky fails when the second turns", "closed")),
  ];
}

export function timelineJob(): Session {
  const third = sketch(PINNED, { title: READS_TITLE, drawing: READS_AGAIN });
  return {
    id: TIMELINE_JOB,
    title: TIMELINE_TITLE,
    turn: { state: "idle" },
    lastTurnAt: ago(10),
    rows: [
      you("t1", 124, `Read ${LINKS.rfc.url} and tell me whether the store can take its clock from outside.`),
      agent("t2", 120, "It can. The failing test is #1887 and the open question is #1890."),
      agent("t3", 108, `The difference between the two clocks is in ${LINKS.instant.url}. The store uses the wall one.`),
      drew("t4", 100, READS),
      you("t5", 97, `This is the failure I mean: ${LINKS.second.url}`),
      shown("t6", 90),
      agent("t7", 81, `A fake clock is the usual answer, ${LINKS.fake.url}`),
      drew("t8", 78, READS_AGAIN, READS_TITLE),
      tool("t9", 70, "show_file /Users/user/armada/docs/spikes/store-clock.md"),
      tool("t10", 60, "Read /tmp/store-clock-failure.png"),
      drew("t11", 52, PINNED, READS_TITLE),
      agent("t12", 44, `The store's last release changed the clock: ${LINKS.changelog.url}`),
      shown("t13", 40),
      tool("t14", 25, "show_file /Users/user/armada/docs/spikes/store-clock.md"),
      agent("t15", 12, "The pull request merged and closed #1887."),
    ],
    attachments: [
      LINKS.rfc,
      issue("1887", "store_flaky fails when the second turns", "closed"),
      issue("1890", "The store clock cannot be injected", "open"),
      LINKS.instant,
      third,
      LINKS.second,
      page(),
      LINKS.fake,
      file(25),
      { kind: "started_to", id: TIMELINE_CHILD, group: "g2" },
      picture,
      pull(1913, { state: "passed" }, "merged"),
      LINKS.changelog,
    ],
    versions: versions(),
    work: {
      railOff: true,
      plan: {
        scope: ["crates/store"],
        approved: true,
        groups: [
          { id: "g1", agent: "rust-engineer", title: "Inject the clock", paths: ["crates/store"], tasks: [] },
          { id: "g2", agent: "rust-engineer", title: "Fake it in the tests", paths: ["crates/store"], tasks: [] },
        ],
      },
    },
  };
}

export function timelineChild(): Session {
  return {
    id: TIMELINE_CHILD,
    title: "Fake the clock in the store tests",
    turn: { state: "idle" },
    lastTurnAt: ago(30),
    rows: [agent("c1", 63, "Started on the group Fake it in the tests.")],
    attachments: [{ kind: "started_by", id: TIMELINE_JOB, group: "g2" }],
  };
}
