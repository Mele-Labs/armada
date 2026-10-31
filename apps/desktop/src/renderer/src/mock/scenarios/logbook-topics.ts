// The topics the mock's sorter gives the Logbook's entries: a first matching word decides, and an entry none match has
// no topic, so the page draws it under Other. `docs/concepts/logbook.md`.

import type { LogbookEntry, LogbookTopic } from "@armada/protocol";

const SORTER: [LogbookTopic, RegExp][] = [
  ["slots", /\bslot|worktree|\blease/i],
  ["testing", /vitest|\btests?\b|\bcheck|type-check|tsc\b|lint|snapshot/i],
  ["git", /\bgit\b|\bgh\b|\bPRs?\b|pull request|\bmerge\b|\bbranch|commit/i],
  ["tools", /start_server|show_sketch|record_plan|\bFleet\b|\bBridge\b|walk|playwright|plugin|mod\b|cockpit|\bhook/i],
  ["mac", /zsh|\bsed\b|\bkill\b|pkill|timeout|\bSSH\b|\/tmp|shell|\bBash\b|brew|Applications/i],
];

/** The entries with the topic the sorter placed each under, where it could. */
export function withTopics(entries: LogbookEntry[]): LogbookEntry[] {
  return entries.map((one) => {
    if (one.topic !== undefined) return one;
    const found = SORTER.find(([, words]) => words.test(`${one.title ?? ""} ${one.text}`));
    return found === undefined ? one : { ...one, topic: found[0] };
  });
}
