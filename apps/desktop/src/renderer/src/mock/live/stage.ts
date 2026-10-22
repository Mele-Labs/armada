// Which stage the page a mock Session is building has reached, shared between the scenario that moves it and the page in the live view's
// frame. Kept in this origin's storage, whose change event reaches the frame without reloading it, as a dev server's hot reload would.

const KEY = "armada.mock.live.plan-card";

/** The scenario moves the page on. */
export function showStage(stage: number): void {
  window.localStorage.setItem(KEY, String(stage));
}

/** The stage the page draws now. */
export function stageNow(): number {
  return Number(window.localStorage.getItem(KEY) ?? 0) || 0;
}

/** Calls `moved` with each stage the scenario moves the page to. */
export function onStage(moved: (stage: number) => void): () => void {
  const heard = (event: StorageEvent) => {
    if (event.key === KEY) moved(stageNow());
  };
  window.addEventListener("storage", heard);
  return () => window.removeEventListener("storage", heard);
}
