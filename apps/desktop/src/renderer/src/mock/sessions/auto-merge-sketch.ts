// The sketch an agent drew of why a pull request in the merge queue still raised a notice (20 Oct 2026): boxes placed
// for 240 by 90 on a three-column grid, with bodies far longer than that. Where boxes piled on each other and arrows ran
// under them. Mock only, as `sketches.ts` is.

import { arrow, box, scene } from "../scenes";

export const AUTO_MERGE_TITLE = "Why a queued pull request still pinged";

export const AUTO_MERGE = scene(
  [
    box("queues", 0, 0, "Agent queues #2645", "gh pr merge --auto, 02:12", { icon: "git-merge", step: 1 }),
    box("auto", 320, 0, "Fleet: auto-merge on", "Told once per pull request; the wait is taken off", { icon: "bot", step: 2 }),
    box("left", 640, 0, "Left the queue", "conflicts, checks failed, 02:23 to 06:36", { icon: "activity", step: 3 }),
    box("again", 0, 170, "Queues it again", "Agent pushes the fix and states a new wait, 11:32", { icon: "git-merge", step: 4 }),
    box("reads", 320, 170, "Fleet reads: in the queue, auto-merge null", "pull_notices.rs:219 reads auto_merge only, and the notice is already told: nothing takes the wait off", { icon: "eye", step: 5 }),
    box("needs", 640, 170, "Needs you shows the row", "Nothing for him to do", { icon: "scale", step: 6 }),
    box("fix", 320, 340, "Fix", "Queue or auto-merge, at every reading: take the wait off, tell the Session only where something came off; agents never state it", { icon: "wrench", step: 7 }),
  ],
  [
    arrow("queues", "auto", undefined, true, 2),
    arrow("auto", "left", undefined, true, 3),
    arrow("auto", "again", undefined, true, 4),
    arrow("again", "reads", undefined, true, 5),
    arrow("reads", "needs", undefined, true, 6),
    arrow("reads", "fix", "becomes", true, 7),
  ],
);
