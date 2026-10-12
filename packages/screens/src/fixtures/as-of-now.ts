import type { JobFixture } from "./fixture";
import { NOW } from "./build/base";

/** An ISO instant anywhere in a fixture, which is the only shape a time takes here. */
const AN_INSTANT = /"(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z)"/g;

/**
 * The same fixture with every instant in it moved so its `NOW` is `now`.
 *
 * **For the mock, which reads the real clock.** A fixture dated last month
 * draws a running Job's run time in hundreds of hours. Every instant moves by
 * the same amount, so a terminal Job's run time is unchanged. The builders
 * keep their fixed times, which tests read — so this is applied where the mock
 * takes a fixture, never inside a builder: a builder built on another one
 * would be moved twice.
 */
export function asOfNow<Fixture extends JobFixture>(fixture: Fixture, now: number = Date.now()): Fixture {
  const moved = now - NOW;
  return JSON.parse(
    JSON.stringify(fixture).replace(AN_INSTANT, (_whole, at: string) =>
      JSON.stringify(new Date(Date.parse(at) + moved).toISOString()),
    ),
  ) as Fixture;
}
