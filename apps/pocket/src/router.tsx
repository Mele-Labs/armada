// Paths without a library: `/`, `/pair`, `/jobs/:id`, `/sessions/:id`, `/dispatch`.

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

export const usePath = () => useSyncExternalStore(subscribe, () => window.location.pathname);

export function go(path: string): void {
  window.history.pushState(null, "", path);
  listeners.forEach((listener) => listener());
}

export function Link({ to, className, children }: { to: string; className?: string; children: ReactNode }) {
  return (
    <a href={to} className={className} onClick={(event) => { event.preventDefault(); go(to); }}>
      {children}
    </a>
  );
}
