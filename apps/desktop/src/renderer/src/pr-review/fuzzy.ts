// Which changed files a few typed letters could mean, best first, as Go to file ranks them. The letters have to appear in the path
// in order; a run of them in a row, one at the start of a word, and one in the file's own name score higher, so `rfi` finds
// `refund_issuer.rb` ahead of a path that only happens to hold an r, an f and an i.

/** Where a word starts: the path's first letter, and any letter after a separator or a lower case letter before a capital. */
const startsWord = (path: string, at: number): boolean =>
  at === 0 ||
  /[/_\-. ]/.test(path[at - 1]!) ||
  (/[a-z]/.test(path[at - 1]!) && /[A-Z]/.test(path[at]!));

/** How well `query` matches `path`, or `undefined` where its letters are not all there in order. Higher is better. */
export function scoreOf(query: string, path: string): number | undefined {
  const wanted = query.toLowerCase().replace(/\s+/g, "");
  if (wanted === "") return 0;
  const lower = path.toLowerCase();
  const name = lower.lastIndexOf("/") + 1;
  // Whether the letters from `index` on are all in the path after `after`, in order.
  const fits = (index: number, after: number): boolean => {
    let from = after + 1;
    for (const letter of wanted.slice(index)) {
      const at = lower.indexOf(letter, from);
      if (at < 0) return false;
      from = at + 1;
    }
    return true;
  };
  let score = 0;
  let from = 0;
  let last = -2;
  for (const [index, letter] of [
    ...wanted,
  ].entries()) {
    // The next one at a word's start wins over the next one anywhere, where both are ahead and the rest still fit after it.
    let at = lower.indexOf(letter, from);
    if (at < 0) return undefined;
    if (at !== last + 1 && !startsWord(path, at)) {
      for (let look = at + 1; look < lower.length; look += 1)
        if (lower[look] === letter && startsWord(path, look) && fits(index + 1, look)) {
          at = look;
          break;
        }
    }
    score += 1;
    if (at === last + 1) score += 4;
    if (startsWord(path, at)) score += 3;
    if (at >= name) score += 2;
    last = at;
    from = at + 1;
  }
  // Of two that match as well, the shorter path is the likelier.
  return score - lower.length / 100;
}

/** The paths `query` matches, best first; ties keep the order they came in. */
export function ranked(query: string, paths: readonly string[]): string[] {
  return paths
    .map((path, at) => ({
      path,
      at,
      score: scoreOf(query, path),
    }))
    .filter(
      (
        one,
      ): one is {
        path: string;
        at: number;
        score: number;
      } => one.score !== undefined,
    )
    .sort((a, b) => b.score - a.score || a.at - b.at)
    .map((one) => one.path);
}
