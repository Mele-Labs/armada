import type { Guide } from "./guide";

/**
 * What the whole tab is, which the three guides already on it never said: 12
 * explains the process list, 13 the disk and 14 the act. Arriving at Pulse
 * opened one of those and answered a question nobody had yet (the owner, 28
 * September 2026).
 *
 * **No `docs/concepts/` page holds Pulse**, the same gap guide 14 names. It is
 * a Bridge surface over Fleet's machine reading, and `job.md` writes up neither.
 */
export const GUIDE_PULSE: Guide = {
  number: 21,
  group: "machine",
  title: "What is Pulse?",
  piece: "pulse.what",
  steps: [
    "Pulse reads what one job is doing to this machine right now.",
    "The band at the top counts what is working: Drones, Checks and Judges.",
    "After the rule it counts what the job has taken: spend against its cap, turns against " +
      "theirs, and processes.",
    "Processes lists each process the job holds, its pid, the checkout it belongs to and what " +
      "it is using.",
    "Worktrees names every checkout on disk, with its branch, its path and its size.",
    "Logs says which files the job is writing and how large they have got.",
    "Refresh asks Fleet to go and look at the machine, and the sentence at the top is what came back.",
    "The last line says when the reading was taken and how often it is taken again while the " +
      "tab is open.",
  ],
};
