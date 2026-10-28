// A Job's agent started two Jobs, each held on its start card. The parent's Needs you has a row for each: its press opens that Job, and
// its Start starts it without opening it. The second Job's request is still being read, so Start shows Starting until the reading ends;
// then its row goes and a linked row lands in the parent's thread. Over `child-job-start`.

import { toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, walk } from "../walk";

const jobs = region("Jobs", { exact: true });
const needs = region("Needs you", { exact: true });
const thread = region("Thread");
const trigger = role("button", "Needs you", { exact: true });
const LOADER = 'Press Start on "Settings loader and the cap"';
const PER_REPO = 'Press Start on "Retry cap per repository"';

const childJobStart = walk("child-job-start", [
  toSessions,
  { press: inside(jobs, button("Debug the retry cap")), say: "A Job whose agent started a Job for each fix" },
  { press: trigger, say: "Needs you" },
  { look: inside(needs, role("listitem", LOADER, { exact: true })), say: "A row for each Job it started, with Start at its end" },
  { press: inside(needs, button(`Open ${LOADER}`, { exact: true })), say: "He presses the row" },
  { look: role("button", "Started by Debug the retry cap", { exact: true }), say: "That Job opens, held on its start card" },
  { press: role("button", "Started by Debug the retry cap", { exact: true }), say: "Back to the Job that started it" },
  { look: inside(needs, role("listitem", PER_REPO, { exact: true })), say: "The second Job's request is still being read" },
  { press: inside(inside(needs, role("listitem", PER_REPO, { exact: true })), button("Start", { exact: true })), say: "Start, from the row" },
  { look: inside(needs, role("img", "Starting", { exact: true })), say: "Starting until the reading ends" },
  { look: inside(thread, role("button", "Retry cap per repository, Started", { exact: true })), say: "It started with the delivery the reading picked. Its row is gone, and a linked row is in the thread" },
  { look: inside(needs, role("listitem", LOADER, { exact: true })), say: "The first Job is still held" },
]);

export { childJobStart as "child-job-start" };
