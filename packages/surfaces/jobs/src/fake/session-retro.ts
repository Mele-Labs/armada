// A Session's retro, for the Retros page: the same items a Job's has, from a Session's own
// record. The pains are the ones the owner met on 7 Oct 2026. Their words are stand-ins for
// what the retro call would write; nothing here is served by Fleet yet.

import type { JobRetro, Lesson, RetroItem, RetroSession } from "@armada/protocol";

/** The Session's retro as the Retro press writes it, and its items as the listing would hold them, all open. */
export function sessionRetro(session: RetroSession, at: string): { retro: JobRetro; lessons: Lesson[] } {
  const t = (minutesBefore: number): string => new Date(Date.parse(at) - minutesBefore * 60_000).toISOString();
  const items: RetroItem[] = [
    {
      id: `${session.id}-1`,
      who: "agent",
      lands_in: "armada",
      title: "A reset waited 40 minutes for an answer",
      what: "The agent asked to run git reset --hard and the Session sat for 40 minutes while you were away.",
      fix: "Tell you when an ask has waited ten minutes, and let the agent carry on with other work meanwhile.",
      statement: "The agent asked to run git reset --hard and the Session sat for 40 minutes while you were away.",
      evidence: ["ask:1"],
      state: "open",
    },
    {
      id: `${session.id}-2`,
      who: "owner",
      lands_in: "armada",
      title: "A search of the whole disk set off privacy prompts",
      what: "The agent ran find over / for pnpm, and macOS asked you to allow access to Photos, Desktop and Downloads, one after another.",
      fix: "Refuse a find that starts at / and tell the agent to search the project folder.",
      statement: "The agent ran find over / for pnpm, and macOS asked you to allow access to Photos, Desktop and Downloads, one after another.",
      evidence: ["tool:1", "note:1"],
      state: "open",
    },
    {
      id: `${session.id}-3`,
      who: "agent",
      lands_in: "armada",
      title: "A commit message was read as a redirect",
      what: "The line Co-Authored-By: … <noreply@…> inside a commit message was read as a message from you, and the agent stopped to answer it.",
      fix: "Read only what you typed as your message, and leave the text of a tool call alone.",
      statement: "The line Co-Authored-By: … <noreply@…> inside a commit message was read as a message from you, and the agent stopped to answer it.",
      evidence: ["correction:1"],
      state: "open",
    },
    {
      id: `${session.id}-4`,
      who: "agent",
      lands_in: "armada",
      title: "A restart of Fleet killed the subagents",
      what: "Fleet restarted while two subagents were mid-review, and both were lost with no result.",
      fix: "Wait for running subagents before a restart, or start them again after it.",
      statement: "Fleet restarted while two subagents were mid-review, and both were lost with no result.",
      evidence: ["restart:1", "subagent:1"],
      state: "open",
    },
    {
      id: `${session.id}-5`,
      who: "owner",
      lands_in: "kit",
      title: "Walks opened in your browser",
      what: "The agent ran open on a walk address, so it came up in your browser instead of in Bridge.",
      fix: "Tell the agent to show a walk with show_window and never to run open.",
      statement: "The agent ran open on a walk address, so it came up in your browser instead of in Bridge.",
      evidence: ["tool:2", "note:2"],
      state: "open",
    },
  ];
  const retro: JobRetro = {
    job_id: session.id,
    session,
    state: "written",
    at,
    items,
    record: {
      asks: [
        {
          cite: "ask:1",
          asked_at: t(52),
          about: "Allow git reset --hard origin/main?",
          answered_at: t(12),
          answer: "Allow once",
          waited_ms: 40 * 60_000,
        },
      ],
      failed_tools: [
        { cite: "tool:1", at: t(46), tool: "Bash", tried: "find / -name pnpm", because: "Operation not permitted, 3 directories" },
        { cite: "tool:2", at: t(30), tool: "Bash", tried: "open http://localhost:5173/?walk=session-retro", because: "Opened in the default browser" },
      ],
      corrections: [{ cite: "correction:1", at: t(21), said: "Co-Authored-By: Claude <noreply@anthropic.com>" }],
      restarts: [{ cite: "restart:1", at: t(9), actor: "fleet", moved: "restarted", said: "Session resumed after Fleet came back" }],
      subagents: [{ cite: "subagent:1", at: t(9), step: "code-review", said: "Stopped at 3m10s when Fleet restarted, no report" }],
      notes: [
        { cite: "note:1", at: t(2), said: "find / -name pnpm hit the privacy prompts before I tried the project folder." },
        { cite: "note:2", at: t(2), said: "I ran open on the walk before I saw show_window." },
      ],
    },
  };
  const lessons: Lesson[] = items.map((item) => ({ job_id: session.id, handle: session.id, session, at, ...item, state: "open" }));
  return { retro, lessons };
}
