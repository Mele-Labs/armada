import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * What a quiet row is, where it is one Armada can name: the harness saying the
 * model is thinking, or the turn's reasoning block. Drawn in words, never as
 * the wire's kind — `system/thinking_tokens` said nothing to the owner, 29 Sep
 * 2026.
 */
export type Thought =
  /** Thinking under way. `tokens` is what this row added, an estimate; absent where none was sent. */
  | { of: "thinking"; tokens?: number }
  /**
   * The turn reasoned. `text` is the reasoning, **a draft awaiting the owner's
   * call**; `withheld` where the vendor redacted it and there is no text to show.
   */
  | { of: "reasoned"; text?: string; withheld?: boolean };

const WORDS: Record<Thought["of"], string> = { thinking: "Thinking", reasoned: "Reasoned" };

/** Where the vendor redacted a turn's reasoning. Plain, and only what is true. */
const WITHHELD = "Withheld by the model's provider";

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

/**
 * A quiet row's words, then its figure trailing. A reasoned row with text
 * shows it on hover and on focus; one withheld says so.
 */
export function ThoughtHead({ thought }: { thought: Thought }) {
  const words = <span className="armada-turns__thought">{WORDS[thought.of]}</span>;
  const hover =
    thought.of === "reasoned" ? (thought.text ?? (thought.withheld ? WITHHELD : undefined)) : undefined;
  return (
    <span className="armada-turns__head">
      {hover === undefined ? words : <Tooltip label={hover}>{words}</Tooltip>}
      {thought.of === "thinking" && thought.tokens !== undefined ? (
        <span className="armada-turns__figure">{tokensSaid(thought.tokens)}</span>
      ) : null}
    </span>
  );
}
