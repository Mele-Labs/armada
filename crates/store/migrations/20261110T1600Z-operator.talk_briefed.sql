-- What the Operator's briefing of the day held, as `ipc::operator_talk::OperatorBriefing` JSON: what
-- moved since the last one, the raises still waiting, what it had in hand and its top line. Null on
-- every line that is not a briefing. Additive.
ALTER TABLE operator_talk ADD COLUMN briefed TEXT;
