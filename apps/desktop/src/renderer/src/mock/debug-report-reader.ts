// What a scenario's `debugReport` readings are to the sheet: one reader, where each opening takes the next reading
// and the last repeats.

import type { DebugRead } from "../debug-report/debug-report";

/** One reading of the report: its text, and how long it takes to arrive. `Infinity` never arrives. */
export type DebugReading = { text: string; after?: number };

export function readingsAsReader(readings: readonly DebugReading[] | undefined): DebugRead | undefined {
  if (readings === undefined || readings.length === 0) return undefined;
  let opened = 0;
  return () => {
    const reading = readings[Math.min(opened, readings.length - 1)]!;
    opened += 1;
    if (reading.after === Infinity) return new Promise<string>(() => undefined);
    return new Promise<string>((resolve) => setTimeout(() => resolve(reading.text), reading.after ?? 0));
  };
}
