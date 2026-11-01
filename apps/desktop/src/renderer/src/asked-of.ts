// What a Session is held on, as the ask cards draw it: the Thread's Needs you panel and a Custom tab's card read one shape.

import type { ComponentProps } from "react";
import type { AskCards } from "@armada/components";
import { helmOfferedOf, reachOfferedOf } from "@armada/screens/src/copy";
import type { Session } from "@armada/screens/src/draft/sessions";

export function askedOf(session: Session): ComponentProps<typeof AskCards>["asked"] {
  if (session.asked === undefined) return undefined;
  return {
    command: session.asked.command,
    ...(session.asked.questions === undefined ? {} : { questions: session.asked.questions }),
    ...(session.asked.offers === undefined
      ? {}
      : { offers: (session.asked.reaches === undefined ? helmOfferedOf : reachOfferedOf)(session.asked.offers).map((one) => ({ id: one.offer, label: one.label, means: one.means })) }),
    ...(session.asked.reaches === undefined ? {} : { reaches: session.asked.reaches }),
    ...(session.asked.reachesTo === undefined ? {} : { reachesTo: session.asked.reachesTo }),
    ...(session.asked.kind === undefined ? {} : { kind: session.asked.kind }),
  };
}
