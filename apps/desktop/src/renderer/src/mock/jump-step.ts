// `jump` as the mock takes it, for a scenario's draft. Fleet will hold the same rule.

/** Where the Job stands after a jump: `at` is the step it lands on, `skipped` every step passed over, those skipped before kept. `undefined` is refused: only a step after the current one can be jumped to. */
export function jumpStep(stand: { at: number | undefined; skipped: readonly number[] }, steps: number, step: number): { at: number; skipped: number[] } | undefined {
  if (stand.at === undefined || !Number.isInteger(step) || step <= stand.at || step >= steps) return undefined;
  const passed = Array.from({ length: step - stand.at }, (_, one) => stand.at! + one);
  const list = [...new Set([...stand.skipped, ...passed])].filter((one) => one !== step).sort((a, b) => a - b);
  return { at: step, skipped: list };
}
