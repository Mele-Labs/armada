// Which layout of the Projects tab is drawn. The mock's takes ride the address: `?layout=a` or `?layout=b`, or a walk named
// `projects-layout-a` or `-b`. Anything else, and Bridge itself, draws the layout the tab has had.

export type Layout = "current" | "a" | "b";

export function layoutOf(search: string = typeof window === "undefined" ? "" : window.location.search): Layout {
  const query = new URLSearchParams(search);
  const asked = query.get("layout") ?? /^projects-layout-([ab])$/.exec(query.get("walk") ?? "")?.[1];
  return asked === "a" || asked === "b" ? asked : "current";
}
