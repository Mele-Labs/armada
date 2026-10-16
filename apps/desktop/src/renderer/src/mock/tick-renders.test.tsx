// What one second of the clock re-renders. The tick used to set state at the top of `App`, so every
// component under it drew again to move a handful of ages; now only the components that read the
// clock do. Counted off React's own commit hook, which has to exist before react-dom loads, hence `vi.hoisted`.

import { expect, test, vi } from "vitest";

import { mount, onScreen, toPlace, unmountAfterEach } from "./testing";

unmountAfterEach();

const { drawn } = vi.hoisted(() => {
  type Fiber = {
    tag: number;
    type: unknown;
    flags: number;
    child: Fiber | null;
    sibling: Fiber | null;
    alternate: Fiber | null;
  };

  const PERFORMED_WORK = 1;
  /** Function, class, forwardRef, memo and simple-memo components, as React tags them. */
  const COMPONENT_TAGS = new Set([0, 1, 11, 14, 15]);

  const drawn: Map<string, number>[] = [];

  /** The names of the components a commit drew again, by walking only what it changed. */
  function walk(fiber: Fiber | null, names: Map<string, number>): void {
    for (let one = fiber; one !== null; one = one.sibling) {
      // Bailed-out subtrees keep their children by identity: nothing in them ran.
      if (one.alternate !== null && one.alternate.child === one.child) continue;
      if (COMPONENT_TAGS.has(one.tag) && (one.flags & PERFORMED_WORK) !== 0) {
        const type = one.type as { displayName?: string; name?: string; render?: { name?: string }; type?: { name?: string } } | null;
        const name = type?.displayName ?? type?.name ?? type?.render?.name ?? type?.type?.name ?? "anonymous";
        names.set(name, (names.get(name) ?? 0) + 1);
      }
      walk(one.child, names);
    }
  }

  (globalThis as { __REACT_DEVTOOLS_GLOBAL_HOOK__?: unknown }).__REACT_DEVTOOLS_GLOBAL_HOOK__ = {
    supportsFiber: true,
    renderers: new Map(),
    inject: () => 1,
    checkDCE: () => undefined,
    onCommitFiberUnmount: () => undefined,
    onPostCommitFiberRoot: () => undefined,
    onCommitFiberRoot: (_id: number, root: { current: Fiber }) => {
      const names = new Map<string, number>();
      walk(root.current, names);
      drawn.push(names);
    },
  };

  return { drawn };
});

const second = (): Promise<void> => new Promise((done) => setTimeout(done, 1100));

/** What two ticks drew, once the mount's own reads have settled. */
async function twoTicks(): Promise<Map<string, number>> {
  await second();
  drawn.length = 0;
  await second();
  await second();
  const total = new Map<string, number>();
  for (const commit of drawn) for (const [name, n] of commit) total.set(name, (total.get(name) ?? 0) + n);
  const count = [...total.values()].reduce((sum, n) => sum + n, 0);
  console.log(`renders over two ticks: ${count} in ${drawn.length} commits`);
  console.log([...total].sort((a, b) => b[1] - a[1]).slice(0, 25).map(([name, n]) => `${n} ${name}`).join("\n"));
  return total;
}

test("a tick on the Cockpit draws nothing: its figures read minutes", async () => {
  mount("every-state");
  await onScreen();
  const total = await twoTicks();
  expect(total.has("App")).toBe(false);
  expect([...total.values()].reduce((sum, n) => sum + n, 0)).toBe(0);
}, 20_000);

test("a tick on the Jobs list moves its seconds and draws no more than the list", async () => {
  mount("every-state");
  await onScreen();
  await toPlace("Jobs");
  const before = document.body.textContent;
  const total = await twoTicks();
  expect(document.body.textContent, "the elapsed figures moved").not.toEqual(before);
  expect(total.has("App")).toBe(false);
  expect(total.has("Shell")).toBe(false);
}, 20_000);
