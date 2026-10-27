// What the page showing offers the palette, first in its results and titled with the page: the Cockpit's acts, Projects',
// or the open Job's own. **Rough for now**: three pages are drawn, and the wiring step reads each page's real acts.

import { Ban, Pause, Play, Plus, GitFork, GitPullRequest } from "lucide-react";
import { SURFACE, type PalettePage } from "@armada/shell";
import type { Session } from "@armada/screens/src/draft/sessions";

/** The id a Job's own row carries in its page block: `job-` and the act. */
export const JOB_PAGE = "job-";

export function pageOf(showing: string, job: Session | undefined): PalettePage | undefined {
  if (job !== undefined) {
    return {
      title: job.title ?? job.id,
      rows: [
        job.paused === true ? { id: `${JOB_PAGE}resume`, label: "Resume", icon: Play } : { id: `${JOB_PAGE}pause`, label: "Pause", icon: Pause },
        { id: `${JOB_PAGE}stop`, label: "Stop", icon: Ban, destructive: true },
        { id: `${JOB_PAGE}fork`, label: "Fork", icon: GitFork },
        { id: `${JOB_PAGE}open-pull`, label: "Open pull request", icon: GitPullRequest },
      ],
    };
  }
  if (showing === SURFACE.overview) {
    return {
      title: "Cockpit",
      rows: [
        { id: "dispatch", label: "Dispatch", icon: Plus },
        { id: "map", label: "Show the map" },
        { id: "timeline", label: "Show the timeline" },
        { id: "needing-you", label: "Go to Jobs needing you" },
      ],
    };
  }
  if (showing === SURFACE.projects) return { title: "Projects", rows: [{ id: "add-task", label: "Add a task", icon: Plus }, { id: "new-project", label: "New project" }] };
  return undefined;
}
