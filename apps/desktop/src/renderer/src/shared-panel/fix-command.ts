// The step a refusal names: Fleet writes the command it wants run between backticks, and the first such span is the one to offer.

/** The first backticked span of `said`, or undefined where it names none. A span across lines is prose, not a command. */
export function commandIn(said: string): string | undefined {
  const command = /`([^`\n]+)`/.exec(said)?.[1]?.trim();
  return command === undefined || command === "" ? undefined : command;
}
