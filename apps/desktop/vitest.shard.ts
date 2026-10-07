// Splits a sharded run by what each file costs, so the walks do not land together.
//
// **Vitest's own `--shard` cuts the files into equal counts after sorting them
// by a hash of the path**, so three of the four `walks-*of4` files shared a shard
// on 6 Oct 2026 (2m58 beside shards that ran 14s and 22s). Here each file has a
// weight, the heaviest goes first into the lightest shard, and the answer is the
// same on every runner: a file is in one shard and no other.
//
// The weights are the seconds measured on CI, rounded; a file not named costs 2.
import { BaseSequencer } from "vitest/node";
import type { TestSpecification } from "vitest/node";

const HEAVY: [RegExp, number][] = [
  [/walks-\d+of\d+\.test\.tsx$/, 50],
  [/every-state\.test\.tsx$/, 20],
  [/src\/main\/connection\.test\.ts$/, 15],
];

function weight(file: TestSpecification): number {
  return HEAVY.find(([pattern]) => pattern.test(file.moduleId))?.[1] ?? 2;
}

export default class WeightedShards extends BaseSequencer {
  override async shard(files: TestSpecification[]): Promise<TestSpecification[]> {
    const { index, count } = this.ctx.config.shard ?? { index: 1, count: 1 };
    const shards = Array.from({ length: count }, () => ({ cost: 0, files: [] as TestSpecification[] }));
    const heaviestFirst = [...files].sort((a, b) => weight(b) - weight(a) || a.moduleId.localeCompare(b.moduleId));
    for (const file of heaviestFirst) {
      const lightest = shards.reduce((least, one) => (one.cost < least.cost ? one : least));
      lightest.files.push(file);
      lightest.cost += weight(file);
    }
    return shards[index - 1]?.files ?? [];
  }
}
