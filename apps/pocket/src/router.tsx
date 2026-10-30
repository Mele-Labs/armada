// Paths without a library. `/` home, `/pair` (`?code=…` from a pairing link, `?origin=…` to pair a Mac
// again), `/dispatch`, and a Job at `/jobs/<Mac origin, encoded>/<id>`, with `/file?path=…` and
// `/sketch?id=…` under it for what the Job showed, and a draft Job at `/drafts/<Mac origin, encoded>/<id>`. `_` for the Mac is a push that did not say which.

import { useSyncExternalStore } from "react";
import type { ReactNode } from "react";

const listeners = new Set<() => void>();
const subscribe = (listener: () => void) => {
  listeners.add(listener);
  window.addEventListener("popstate", listener);
  return () => {
    listeners.delete(listener);
    window.removeEventListener("popstate", listener);
  };
};

export const usePath = () => useSyncExternalStore(subscribe, () => window.location.pathname + window.location.search);

export function go(path: string, replace = false): void {
  if (replace) window.history.replaceState(null, "", path);
  else window.history.pushState(null, "", path);
  listeners.forEach((listener) => listener());
}

export type Route =
  | { screen: "home" }
  | { screen: "pair"; code: string; origin?: string }
  | { screen: "dispatch" }
  | { screen: "job"; origin: string | undefined; id: string }
  | { screen: "draft"; origin: string; id: string }
  | { screen: "file"; origin: string; id: string; path: string }
  | { screen: "sketch"; origin: string; id: string; sketch: string };

export const jobPath = (origin: string, id: string) => `/jobs/${encodeURIComponent(origin)}/${encodeURIComponent(id)}`;
export const draftPath = (origin: string, id: string) => `/drafts/${encodeURIComponent(origin)}/${encodeURIComponent(id)}`;
export const filePath = (origin: string, id: string, path: string) => `${jobPath(origin, id)}/file?path=${encodeURIComponent(path)}`;
export const sketchPath = (origin: string, id: string, sketch: string) => `${jobPath(origin, id)}/sketch?id=${encodeURIComponent(sketch)}`;
export const pairPath = (origin?: string) => (origin === undefined ? "/pair" : `/pair?origin=${encodeURIComponent(origin)}`);

export function routeOf(whole: string): Route {
  const url = new URL(whole, "http://pocket");
  const query = url.searchParams;
  if (url.pathname === "/pair") {
    const origin = query.get("origin");
    return { screen: "pair", code: query.get("code") ?? "", ...(origin === null ? {} : { origin }) };
  }
  if (url.pathname === "/dispatch") return { screen: "dispatch" };
  const draft = /^\/drafts\/([^/]+)\/([^/]+)$/.exec(url.pathname);
  if (draft !== null) return { screen: "draft", origin: decodeURIComponent(draft[1]!), id: decodeURIComponent(draft[2]!) };
  const job = /^\/jobs\/([^/]+)\/([^/]+)(?:\/(file|sketch))?$/.exec(url.pathname);
  if (job !== null) {
    const mac = decodeURIComponent(job[1]!);
    const id = decodeURIComponent(job[2]!);
    if (job[3] === "file" && mac !== "_") return { screen: "file", origin: mac, id, path: query.get("path") ?? "" };
    if (job[3] === "sketch" && mac !== "_") return { screen: "sketch", origin: mac, id, sketch: query.get("id") ?? "" };
    return { screen: "job", origin: mac === "_" ? undefined : mac, id };
  }
  return { screen: "home" };
}

export function Link({ to, className, children }: { to: string; className?: string; children: ReactNode }) {
  return (
    <a href={to} className={className} onClick={(event) => { event.preventDefault(); go(to); }}>
      {children}
    </a>
  );
}
