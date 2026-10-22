// Which of the three drawings of Progress the mock shows, read from `?progress=`: `zones` (Now, Plan and Issues as three named zones), `spine`
// (the plan is the spine, and the turn's account is inside its current group) and `folded` (Now pinned, the plan and the issues folded to a row
// each). A mock-only choice, made once he has looked: the one picked stays and this file goes.

export type ProgressTake = "zones" | "spine" | "folded";

const TAKES: readonly ProgressTake[] = ["zones", "spine", "folded"];

export function progressTake(): ProgressTake {
  if (typeof window === "undefined") return "zones";
  const asked = new URLSearchParams(window.location.search).get("progress");
  return TAKES.find((one) => one === asked) ?? "zones";
}
