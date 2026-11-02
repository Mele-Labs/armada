-- breaking
-- Autopilot is the one overnight control, so Sleep's tables go. A night's answer the person has
-- not reviewed moves into `autopilot_decisions` as a card to review. Sleep had no word for the
-- repository, why or kind: the repository is the answering session's own (hosted row, else the
-- ledger's), the kind `recommended` since that is all Sleep answered, the reason empty, and the time
-- the night began. A row whose session is gone has no repository to name, and `manifest_id` is
-- NOT NULL, so it is dropped. Reviewed answers, and what the night held, landed and showed, go.
INSERT OR IGNORE INTO autopilot_decisions
    (id, manifest_id, session_id, who, kind, asked, chose, because, at, reviewed, corrected)
SELECT r.id, COALESCE(h.manifest_id, s.manifest_id), r.session_id, r.who, 'recommended', r.text,
       COALESCE(r.chose, ''), '',
       COALESCE((SELECT since FROM sleep_state), strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
       0, r.corrected
FROM sleep_rows r
LEFT JOIN hosted_sessions h ON h.session_id = r.session_id
LEFT JOIN sessions s ON s.id = r.session_id
WHERE r.kind = 'decided' AND r.reviewed = 0 AND COALESCE(h.manifest_id, s.manifest_id) IS NOT NULL
ORDER BY r.position;

DROP TABLE sleep_rows;
DROP TABLE sleep_state;
