// `skip_session_step` as the mock takes it, for the fake Fleet's records and for a scenario's draft alike. Fleet will hold the same rule.

/** Where the Job stands after a skip or an undo: `at` is the current step, `skipped` the steps passed over. `undefined` is refused, as the last, delivering step is. */
export function skipStep(stand: { at: number | undefined; skipped: readonly number[] }, steps: number, step: number, skipped: boolean): { at: number | undefined; skipped: number[] } | undefined {
  if (!Number.isInteger(step) || step < 0 || step >= steps - 1) return undefined;
  const list = skipped ? [...new Set([...stand.skipped, step])].sort((a, b) => a - b) : stand.skipped.filter((one) => one !== step);
  let at = stand.at;
  if (skipped) {
    // The current step moves on to the next one not skipped. The last step is never skipped, so it stops there.
    while (at !== undefined && at < steps - 1 && list.includes(at)) at += 1;
  } else if (at !== undefined && step < at) {
    // An undo reverses the skip: where only skipped steps lie between the step and the current one, the Job goes back to it.
    let between = true;
    for (let one = step + 1; one < at; one += 1) between &&= list.includes(one);
    if (between) at = step;
  }
  return { at, skipped: list };
}
