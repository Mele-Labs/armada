// Which arrangement of the Session's panels is drawn. Mock only: the walk `oneSessionView` offers two, `&take=a` and
// `&take=b`, for the owner to choose between; the one he keeps becomes the only one and this goes.

export type Take = "a" | "b";

export function takeOf(): Take {
  return new URLSearchParams(window.location.search).get("take") === "b" ? "b" : "a";
}
