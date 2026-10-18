import type { Guide } from "./guide";

/**
 * What the word means, for a person who has met it on a badge and nowhere
 * else. Written from `docs/concepts/session.md`.
 */
export const GUIDE_DRONE: Guide = {
  number: 18,
  group: "run",
  title: "What is a drone?",
  piece: "run.drone",
  concept: "docs/concepts/session.md",
  steps: [
    "A Drone is one agent instance.",
    "A job's own agent is one, and each subagent it starts is another.",
    "It works in a slot its job holds, with the tools Armada gives it.",
    "When it ends, its conversation ends with it.",
    "The slot, the branch and the uncommitted work stay with the job. The transcript and the " +
      "context do not.",
    "So whatever the next Drone needs is something that was written down.",
    "A Drone moves nothing on its own. Fleet reads what it did.",
  ],
};
