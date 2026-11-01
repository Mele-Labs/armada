// The design tokens the note card's tools offer, by name, from `packages/tokens/src`. Their values are
// read off the page where they are used, so a theme's are the ones shown and recorded.

/** The type scale, `typography.css`. */
export const SIZES = ["--text-2xs", "--text-xs", "--text-sm", "--text-base", "--text-lg", "--text-xl", "--text-2xl"] as const;

/** Text colours, `colors.css` and `status.css`. */
export const COLOURS = [
  "--fg-default",
  "--fg-muted",
  "--fg-subtle",
  "--fg-inverse",
  "--accent",
  "--status-running",
  "--status-awaiting-review",
  "--status-escalated",
  "--status-completed-success",
  "--status-completed-failed",
  "--status-rejected",
] as const;

/** Grounds, `colors.css` and `status.css`. */
export const FILLS = [
  "--bg-base",
  "--bg-sunken",
  "--bg-raised",
  "--bg-overlay",
  "--bg-hover",
  "--accent-muted",
  "--accent",
  "--status-running-bg",
  "--status-awaiting-review-bg",
  "--status-completed-success-bg",
  "--status-completed-failed-bg",
] as const;

/** The space scale, `spacing.css`. */
export const SPACES = ["--space-1", "--space-2", "--space-3", "--space-4", "--space-6", "--space-8", "--space-12"] as const;

/** What a scale's button reads: the token less its family, `--text-lg` as `lg`. */
export const step = (token: string): string => token.replace(/^--(text|space)-/, "");
