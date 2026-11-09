-- A Job's `all_done` held at an open gate: when the agent first said it, and its summary. One row per
-- Job, removed when Fleet finishes the Job as the last gate closes, or when the agent's done stops
-- standing.
CREATE TABLE session_gate_holds (
    session_id TEXT NOT NULL PRIMARY KEY,
    since      TEXT NOT NULL,
    summary    TEXT
);
