// The held calls' registration: the answer is the only member, and a mock Fleet takes it.

import type { Outcome } from "@armada/protocol";

import type { QuestionsApi, QuestionsState } from "../../../../shared/api/questions";
import { QUESTIONS_NOTHING_YET } from "../../../../shared/api/questions";
import type { Slice } from "../fake-context";

const OK: Outcome = {
  ok: true,
};

export const questions: Slice<QuestionsApi, QuestionsState> = {
  name: "questions",
  state: QUESTIONS_NOTHING_YET,
  scenarios: () => [],
  api: () => ({
    answerCall: async () => OK,
  }),
};
