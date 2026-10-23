// The `not-a-question` scenario in the take the address asks for: `?take=thumbs` or `?take=menu`, else the button. `../not-a-question-scenario.ts`.

import type { Scenario } from "../moment";
import { notAQuestionOver, type Take } from "../not-a-question-scenario";

const asked = new URLSearchParams(window.location.search).get("take");

export const notAQuestion: Scenario = notAQuestionOver("not-a-question", asked === "thumbs" || asked === "menu" ? (asked satisfies Take) : "button");
