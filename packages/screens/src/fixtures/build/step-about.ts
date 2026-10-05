// What each shipped step says it does: `about` in `.armada/workflows/*.json`,
// carried so a mock Fleet serves what the real one does. Fixtures read it by
// workflow and step id, and a step the files do not declare reads nothing.

const ABOUT: Record<string, Record<string, string>> = {
  bug: {
    plan: "Reads the report and the code, finds the cause, and writes down the fix as tasks. Nothing is changed yet. The tasks are what Implement works from.",
    implement: "Makes the fix from the plan, one Drone per task, and runs the repository's checks on the result. The change is held in the Job's branch.",
    handoff: "Puts the finished change in front of you with what was checked. You decide whether it goes out.",
  },
  code_review: {
    read: "Reads the pull request's diff and writes down what it changes. The pull request itself is only read. The notes are what Assess works from.",
    assess: "Weighs the change against the notes and writes the review. You read it before it goes anywhere.",
    deliver: "Delivers the review you approved. No code is landed.",
  },
  design_plan: {
    draft: "Writes the design as a document and records the plan behind it. No code is written.",
    present: "Shows you the document. You approve it or send it back with notes for another draft.",
  },
  epic: {
    plan: "Splits the milestone or issue list into Jobs and proposes them. They wait until you approve, and approving starts the whole wave.",
    roll_up: "Writes up what the wave's Jobs did as one document. You read it before the epic closes.",
  },
  feature: {
    plan: "Reads the request and the code, and writes down the change as tasks. Nothing is changed yet. The tasks are what Implement works from.",
    implement: "Builds the change from the plan, one Drone per task, and runs the repository's checks on the result.",
    tests: "Writes tests for the new behaviour and runs the repository's checks again.",
    handoff: "Puts the finished change in front of you with what was checked. You decide whether it goes out.",
  },
  prototype: {
    frame: "Writes three lines on what will be tried and how, before any code. Build works from them.",
    build: "Builds a rough version to answer the question. The repository's walk server starts so you can use it. You decide whether it is enough.",
    write_up: "Writes up what was tried and what it showed. The code is not landed.",
  },
  refactor: {
    plan: "Reads the code and writes down the restructure as tasks. Nothing is changed yet. The tasks are what Restructure works from.",
    implement: "Moves and renames code from the plan, one Drone per task, and runs the repository's checks to confirm behaviour has not changed.",
    handoff: "Puts the finished change in front of you with what was checked. You decide whether it goes out.",
  },
  revert: {
    revert: "Takes the named change back out and nothing else, then runs the repository's checks.",
    handoff: "Summarises what was taken out. You decide whether it goes out.",
  },
};

/** The `about` line `.armada/workflows/<workflow>.json` gives a step, as the field it is served in. */
export function aboutOf(workflow: string, step: string): { about?: string } {
  const about = ABOUT[workflow]?.[step];
  return about === undefined ? {} : { about };
}
