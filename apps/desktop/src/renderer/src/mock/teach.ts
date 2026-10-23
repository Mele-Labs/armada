// Mock-only: Fleet's read of what the owner taught, answered from two canned inputs. A Jira key becomes a link rule and a
// note for agents; a habit becomes a note alone. Any other words are read as a note alone.

import type { TeachSource } from "../teach";

const READ_MS = 1500;

export function createMockTeaching(): TeachSource {
  return {
    read: ({ words, say }) =>
      new Promise((resolve) =>
        window.setTimeout(
          () =>
            resolve(
              /\bSTORE-\d+\b/.test(words) || /https?:\/\//.test(say)
                ? {
                    link: { example: "STORE-1234", pattern: "STORE-\\d+", address: "https://myproject.jira.com/{match}" },
                    note: "Jira keys look like STORE-1234. Cite them as links.",
                  }
                : { note: "Propose an idea as a sketch with a short list underneath, not prose." },
            ),
          READ_MS,
        ),
      ),
  };
}
