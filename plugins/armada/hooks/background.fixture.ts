// What Claude Code said about background tasks, verbatim from a live hosted session on 16 Oct 2026
// (paths cut with `...` as they were handed over). **No Monitor sample has been recorded.**

/** The Bash result of a call with `run_in_background`. */
export const BASH_LAUNCH =
  'Command running in background with ID: b3wd556q5. Output is being written to: /private/tmp/claude-501/-Users-nickmele-Development-armada--armada-slots-slot-12/aa994f6f-56ce-4b09-9e06-66342d14548f/tasks/b3wd556q5.output. You will be notified when it completes. To check interim output, use Read on that file path.'

/** The notice of a command that completed, a user message inside a system-reminder with other text around it. */
export const NOTICE_COMPLETED =
  '<system-reminder>\nA background task finished.\n<task-notification>\n<task-id>byry3zett</task-id>\n<tool-use-id>toolu_01MxedjnFGy3FJgYQX1xC9U3</tool-use-id>\n<output-file>/private/tmp/claude-501/.../tasks/byry3zett.output</output-file>\n<status>completed</status>\n<summary>Background command "Wait for required ci on PR 2438" completed (exit code 0)</summary>\n</task-notification>\n</system-reminder>'

/** The notice of a command stopped at its background time limit. */
export const NOTICE_KILLED =
  '<task-notification>\n<task-id>bdjviwn7c</task-id>\n<tool-use-id>toolu_01YSsexZihjGwrr7QUkKGbiz</tool-use-id>\n<output-file>/private/tmp/claude-501/.../tasks/bdjviwn7c.output</output-file>\n<status>killed</status>\n<summary>Background command "Serve the Bridge mock from slot 3" was stopped after reaching its background time limit</summary>\n</task-notification>'

/** A subagent's completion uses the same envelope, with an agent id. */
export const NOTICE_SUBAGENT =
  '<task-notification>\n<task-id>a296998e19407a04f</task-id>\n<status>completed</status>\n<summary>Agent "Read the CI history" finished</summary>\n</task-notification>'

/** The result of a `TaskStop`. */
export const TASK_STOPPED =
  '{"message":"Successfully stopped task: b3wd556q5 (pnpm -C apps/desktop exec vite ...)","task_id":"b3wd556q5","task_type":"local_bash","command":"pnpm -C apps/desktop exec vite ..."}'
