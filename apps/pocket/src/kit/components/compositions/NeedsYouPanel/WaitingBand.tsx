/** One thing a Job waits on that is not him: a watch, its pull request in the queue or set to merge, or its pull request's Checks running. */
export type WaitingLine = {
  kind: "watch" | "merge" | "checks";
  said: string;
};
