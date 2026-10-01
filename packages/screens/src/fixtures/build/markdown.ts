// What an agent writes, written in markdown, at every surface that draws it:
// a Drone's sentences in its turns and its log, the question it asks and its
// answers, and a question in the dock from the Drone and from the Judge.
//
// **`executingSequential`, with its Drones writing as a model writes.**
// Emphasis, a list and a name in code in every sentence, so each surface is
// seen drawing all three — and a surface that drew the characters instead is
// plain to see.

import type { JudgeQuestion, QuestionInFlight, Turn } from "@armada/protocol";

import type { DroneView } from "../../draft";
import type { Outstanding } from "../../outstanding";
import type { JobFixture } from "../fixture";
import type { ArcMoment } from "./arc-base";
import { ARC_JOB_ID, ARC_NOW, arcStep, featureWorkflow } from "./arc-base";
import { executingSequential } from "./arc-executing";
import { said } from "./base";
import { lightFixture } from "./light";

/** What T1's Drone said on finishing, in place of its plain closing line. */
const FINISHED =
  "Done. **The read answers** in one call:\n\n" +
  "- `running_rows` carries Drones, Checks and Judge calls\n" +
  "- the stat reads it without a poll";

/** What T5's Drone says last, which the Job's log ends on. */
const WORKING =
  "Drawing the four lists. **Their order** is fixed:\n\n" +
  "- `RunningList` keeps Drones first\n" +
  "- an empty list still draws its heading";

/** The question T5's Drone stopped to ask, and its two answers. */
const ASKING: QuestionInFlight = {
  question_id: "q_empty_list",
  step_id: "implement",
  asked_at: "2026-09-22T10:19:00Z",
  question:
    "Should an empty list keep its **heading**?\n\n" +
    "- `RunningList` hides it today\n" +
    "- the plan's brief says draw it",
  options: [
    { label: "Keep it", consequence: "Draw the heading over an empty list. **Every list** keeps its place." },
    {
      label: "Hide it",
      consequence: "Hide the heading:\n\n- `RunningList` stays as it is\n- the lists **move up** when one empties",
    },
  ],
};

/** The second Job, at its gate, where the Judge refused a criterion and asks. */
const OTHER_ID = "01M2D9C3ZM001MARKDOWN0002";

const REFUSED: JudgeQuestion = {
  step_id: "handoff",
  criterion_id: "selectors_cover_the_empty_store",
  question: "Does every selector answer for a store nothing has written to yet?",
  expected: "A case for the empty store beside each selector",
  produced: "Two of the six selectors are exercised only against a filled store",
  consequence:
    "A first launch reads **undefined** through `selectColumns`:\n\n" +
    "- the panel draws no columns\n" +
    "- the order resets on the first save",
  asked_at: "2026-09-22T10:48:00Z",
};

/** A transcript whose last sentence says `text` instead. */
function endingOn(transcript: readonly Turn[], text: string): Turn[] {
  const last = transcript.map((turn) => turn.saw.event).lastIndexOf("said");
  return transcript.map((turn, at) =>
    at === last && turn.saw.event === "said" ? { ...turn, saw: { ...turn.saw, text } } : turn,
  );
}

/** T1's Drone closing in markdown; every other Drone as it was. */
function written(drones: readonly DroneView[]): DroneView[] {
  return drones.map((drone) =>
    drone.task === "T1" && drone.transcript !== undefined
      ? { ...drone, transcript: endingOn(drone.transcript, FINISHED) }
      : drone,
  );
}

/** The Job, stopped on T5's question, with T5's turns on its log. */
function asking(was: JobFixture, drones: readonly DroneView[]): JobFixture {
  const job = { ...was.job, asking: true };
  const five = drones.find((drone) => drone.task === "T5")?.transcript ?? [];
  const watched =
    was.watched.state === "read"
      ? { ...was.watched, detail: { ...was.watched.detail, job, asking: ASKING } }
      : was.watched;
  return {
    ...was,
    name: "running — its Drones write markdown, and one asks a question",
    job,
    watched,
    observed: {
      state: "watching",
      jobId: ARC_JOB_ID,
      turns: { live: true, skipped: 0, missed: 0, rows: [...five, said("implement", "2026-09-22T10:18:40Z", WORKING)] },
    },
  };
}

/** A second Job whose Judge refused a criterion and is asking. */
function refused(): JobFixture {
  return lightFixture(
    {
      id: OTHER_ID,
      handle: "41-cover-the-empty-store",
      title: "Cover the empty store in every selector",
      status: "awaiting_review",
      workflow: featureWorkflow(),
      at: "handoff",
      steps: [
        arcStep("plan", "Plan the change", 1),
        arcStep("implement", "Implement", 2),
        arcStep("tests", "Write tests", 3),
        arcStep("handoff", "Review the change", 4),
      ],
      says: "awaiting_review — the Judge refused a criterion and asks",
      created_at: "2026-09-22T09:20:00Z",
      started_at: "2026-09-22T09:22:00Z",
      branch: "armada/41-cover-the-empty-store",
      detail: { judge_question: REFUSED },
    },
    ARC_NOW,
  );
}

export function agentText(): ArcMoment {
  const was = executingSequential();
  const drones = written(was.draft.drones ?? []);
  const questions: Outstanding[] = [
    { kind: "drone", job_id: ARC_JOB_ID, asking: ASKING },
    { kind: "judge", job_id: OTHER_ID, question: REFUSED },
  ];
  return {
    ...was,
    name: "agent-text",
    says: "A Drone and a Judge writing markdown, at every surface that draws it",
    fixtures: [asking(was.fixtures[0]!, drones), refused()],
    questions,
    draft: { ...was.draft, drones },
  };
}
