// How the run canvas draws a plan's tasks, for the owner to compare in the
// mock (5 Oct 2026): `chains` hangs each group's tasks under its card,
// `clusters` makes each group a dashed Cluster holding them. **One switch**, so
// the one that loses is deleted by following its reads: `?plan=clusters`
// picks the clusters, a walk may set it, and the default is the chains.

export type PlanLayout = "chains" | "clusters";

let set: PlanLayout | undefined;

/** A walk's own choice, over the address's. `undefined` hands it back. */
export function setPlanLayout(layout: PlanLayout | undefined): void {
  set = layout;
}

export function planLayout(): PlanLayout {
  if (set !== undefined) return set;
  const asked = new URLSearchParams(globalThis.location?.search ?? "").get("plan");
  return asked === "clusters" ? "clusters" : "chains";
}
