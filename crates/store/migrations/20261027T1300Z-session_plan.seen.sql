-- Whether the agent said the person would see the change, so a walk follows approval.
-- Zero for every plan recorded before this, which said nothing.
ALTER TABLE session_plan ADD COLUMN seen INTEGER NOT NULL DEFAULT 0;
