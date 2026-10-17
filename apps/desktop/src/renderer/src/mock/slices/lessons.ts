// Retros and Lessons: a Session's retro, the listing, and the owner's answer to an item. Each
// answers from the scenario's retros and Lessons, and an answer moves the item for the next read.

import type { Lesson, LessonAnswer, RetroItem } from "@armada/protocol";
import { answered, askedAbout, listed, reviewed, sessionStarted } from "@armada/screens/src/lessons/lessons-fleet";

import type { LessonsApi } from "../../../../shared/api/lessons";
import type { Slice } from "../fake-context";

export const lessons: Slice<LessonsApi, Record<string, never>> = {
  name: "lessons",
  state: {},
  api: (scenario, fleet) => {
    const { kit, refused } = fleet;

    /** The retro items as answers left them. */
    let held: Lesson[] = scenario.lessons ?? [];

    /** One answer to one item. */
    function answerLesson(id: string, answer: "agree" | "disagree"): LessonAnswer {
      const taken = answered(held, id, answer);
      held = taken.lessons;
      // Update Kit adds the command Fleet says it applied, as that retro item's.
      const applied = taken.answer.ok ? taken.answer.lesson.applied : undefined;
      if (applied !== undefined && !kit.get().some((one) => one.run === applied.command)) {
        kit.set([...kit.get(), { run: applied.command, source: "retro_item", lesson_id: id }]);
      }
      return taken.answer;
    }

    return {
      readRetro: async (subject) => {
        // A Session's numbered retro is held under `{session}-r{n}`; its newest under the Session's id. An unknown `n` is refused.
        const retro = scenario.retros?.[subject.n !== undefined ? `${subject.id}-r${subject.n}` : subject.id];
        if (retro === undefined) return refused(`/sessions/${subject.id}/retro${subject.n === undefined ? "" : `?n=${subject.n}`}`);
        // Each item stands as the answers left it, as Fleet's `state` says. An old item has none.
        const standing = (item: RetroItem): RetroItem => {
          const now = item.state === undefined ? undefined : held.find((one) => one.id === item.id);
          if (now === undefined) return item;
          return {
            ...item,
            state: now.state,
            ...(now.session_proposed === undefined ? {} : { session_proposed: now.session_proposed }),
            ...(now.applied === undefined ? {} : { applied: now.applied }),
          };
        };
        return { ok: true, retro: { ...retro, items: retro.items?.map(standing) } };
      },
      readLessons: async (view) => ({ ok: true, lessons: listed(held, view) }),
      agreeLesson: async (id) => answerLesson(id, "agree"),
      disagreeLesson: async (id) => answerLesson(id, "disagree"),
      startLessonSession: async (id) => {
        const taken = sessionStarted(held, id);
        held = taken.lessons;
        return taken.read;
      },
      reviewLessons: async () => reviewed(held),
      askLesson: async (id, question, history) => askedAbout(held, id, { question, history }),
    };
  },
};
