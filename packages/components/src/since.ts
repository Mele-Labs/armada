/** Time since `at`, as "4m", "2h" or "3d"; nothing for a stamp that will not parse. */
export function since(at: string, now: number): string | undefined {
  const then = Date.parse(at);
  if (Number.isNaN(then)) return undefined;
  const minutes = Math.max(0, Math.floor((now - then) / 60_000));
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  return hours < 24 ? `${hours}h` : `${Math.floor(hours / 24)}d`;
}
