/** One comment the person may hand to the Job to weigh. */
export type EvaluatedComment = {
  id: string;
  author: string;
  body: string;
  /** `path:line`, or `path` for a comment on a whole file. Absent on the conversation. */
  where?: string;
};
