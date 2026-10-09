// The one module the screens read their Jobs and Sessions from. Fixtures today; the
// Phone Gateway client replaces it in #1998 and nothing else in the app changes.

import { APPROVAL, BLOCKED, REVIEW, RUNNING, DONE } from "./fixtures";
import type { PocketJob } from "./fixtures";

export * from "./fixtures";

export const jobById = (id: string): PocketJob | undefined =>
  [...BLOCKED, APPROVAL, REVIEW, ...RUNNING, ...DONE].find((job) => job.id === id);
