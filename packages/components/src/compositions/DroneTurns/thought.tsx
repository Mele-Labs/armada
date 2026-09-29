/**
 * What a quiet row is, where it is one Armada can name: the harness saying the
 * model is thinking. Never drawn as the wire's kind — `system/thinking_tokens`
 * said nothing to the owner, 29 Sep 2026.
 */
export type Thought =
  /** Thinking under way. `tokens` is what this row added, an estimate; absent where none was sent. */
  { of: "thinking"; tokens?: number };

/** `~400 tokens`, `~1,800 tokens`, `~1 token`. The tilde because every figure is the harness's estimate. */
export function tokensSaid(tokens: number): string {
  return `~${tokens.toLocaleString("en-US")} ${tokens === 1 ? "token" : "tokens"}`;
}

/** Every token a run's rows added, or undefined where none carries a count. */
export function tokensOf(thoughts: readonly (Thought | undefined)[]): number | undefined {
  const counted = thoughts.flatMap((one) =>
    one?.of === "thinking" && one.tokens !== undefined ? [one.tokens] : [],
  );
  return counted.length === 0 ? undefined : counted.reduce((sum, one) => sum + one, 0);
}
