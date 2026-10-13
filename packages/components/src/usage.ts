/** How full a limit is, by the figure as it is drawn: normal below 80, warning from 80, danger from 95. */
export type UsageLevel = "normal" | "warning" | "danger";

export function usageLevel(percent: number): UsageLevel {
  return percent >= 95 ? "danger" : percent >= 80 ? "warning" : "normal";
}
