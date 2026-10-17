// The guided review's queue: the model's order resolved against the Lessons the
// page already holds. Pure, so the order of events is tested without a screen.
//
// **Nothing here answers an item.** The review only decides what to show and
// when; an answer is `useAnswers`' and means what it means on the list.

import type { Lesson } from "@armada/protocol";

import type { LessonReview } from "./review-wire";

/** One card in the queue, with the duplicates it stands for. */
export type QueueEntry = {
  lesson: Lesson;
  reason: string;
  /** The duplicates Fleet named that the page still holds. */
  merged: Lesson[];
};

/** One item the review left out, with the model's reason. */
export type AsideEntry = { lesson: Lesson; why: string };

/**
 * The review against the page's open items. **An id the page does not hold is
 * left out** (answered since, or read again): the review was made a moment
 * ago, and a card the page cannot draw is no card.
 */
export function resolved(review: LessonReview, lessons: readonly Lesson[]): { queue: QueueEntry[]; aside: AsideEntry[] } {
  const held = new Map(lessons.map((one) => [one.id, one]));
  const queue = review.entries.flatMap((entry) => {
    const lesson = held.get(entry.lesson_id);
    if (lesson === undefined) return [];
    const merged = entry.merged_ids.flatMap((id) => {
      const twin = held.get(id);
      return twin === undefined ? [] : [twin];
    });
    return [{ lesson, reason: reasonOf(entry.reason), merged }];
  });
  const aside = review.set_aside.flatMap((one) => {
    const lesson = held.get(one.lesson_id);
    return lesson === undefined ? [] : [{ lesson, why: one.why }];
  });
  return { queue, aside };
}

/** Fleet appends an item the model left out of every list with `not ranked`. Say that in words. */
export function reasonOf(reason: string): string {
  return reason.trim().toLowerCase() === "not ranked" ? "The review did not rank this one." : reason;
}

/**
 * How long ago the review was made, for `Reviewed 12 minutes ago`. **Whole units, rounded down**, so
 * a review a minute and a half old reads as one minute. Under a minute is `just now`.
 */
export function ageWords(ms: number): string {
  const minutes = Math.floor(Math.max(0, ms) / 60_000);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes} ${minutes === 1 ? "minute" : "minutes"} ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} ${hours === 1 ? "hour" : "hours"} ago`;
  const days = Math.floor(hours / 24);
  return `${days} ${days === 1 ? "day" : "days"} ago`;
}
