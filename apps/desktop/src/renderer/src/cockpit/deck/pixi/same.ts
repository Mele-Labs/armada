// Whether two scenes are the same picture. The page hands the canvas a fresh scene each time the clock it reads moves (every five seconds),
// with every list and map in it rebuilt and nothing in them changed; telling that apart here is what lets a map nobody touches draw
// nothing at all. The `now` a build carries is the clock the page read; the canvas does not draw it, so it is not compared.

export function same(a: unknown, b: unknown, depth = 0): boolean {
  if (Object.is(a, b)) return true;
  if (typeof a !== "object" || typeof b !== "object" || a === null || b === null || depth > 40) return false;
  if (a instanceof Map) {
    if (!(b instanceof Map) || a.size !== b.size) return false;
    for (const [key, value] of a) if (!b.has(key) || !same(value, b.get(key), depth + 1)) return false;
    return true;
  }
  if (a instanceof Set) {
    if (!(b instanceof Set) || a.size !== b.size) return false;
    for (const value of a) if (!b.has(value)) return false;
    return true;
  }
  if (Array.isArray(a) !== Array.isArray(b)) return false;
  const [ka, kb] = [Object.keys(a), Object.keys(b)];
  if (ka.length !== kb.length) return false;
  for (const key of ka) if (key !== "now" && !same((a as Record<string, unknown>)[key], (b as Record<string, unknown>)[key], depth + 1)) return false;
  return true;
}
