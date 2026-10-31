// Each browser test file gets a localStorage of its own.
//
// Files run side by side in one browser origin, so they share its storage, and
// a write or `forgetAllKept()` in one fires `storage` in the others. `keep.ts`
// reads that as another Bridge window and drops what it held, so a test's
// typed request can empty mid-test while a neighbour clears its own (the
// Dispatch queue test on main, 31 Oct 2026). With storage of its own, a
// neighbour's `storage` event only re-reads words that are still here.

class OwnStorage implements Storage {
  readonly held = new Map<string, string>();
  get length() {
    return this.held.size;
  }
  clear() {
    this.held.clear();
  }
  getItem(key: string) {
    return this.held.get(key) ?? null;
  }
  key(index: number) {
    return [...this.held.keys()][index] ?? null;
  }
  removeItem(key: string) {
    this.held.delete(key);
  }
  setItem(key: string, value: string) {
    this.held.set(key, String(value));
  }
}

const own = new OwnStorage();
// `Object.keys(localStorage)` lists stored keys, as the real one does.
const storage = new Proxy(own, {
  ownKeys: (target) => [...target.held.keys()],
  getOwnPropertyDescriptor: (target, key) =>
    typeof key === "string" && target.held.has(key)
      ? { configurable: true, enumerable: true, value: target.getItem(key) }
      : undefined,
});

Object.defineProperty(window, "localStorage", { configurable: true, value: storage });
