-- The model and effort a person chose for one Job's Commander, over the machine's `models.commander`
-- and `effort.commander`. NULL is nobody having chosen. A chosen effort is spelled as the setting
-- spells it, "harness default" included, so choosing the harness's own is not the same as choosing
-- nothing.
ALTER TABLE jobs ADD COLUMN commander_model_override TEXT;
ALTER TABLE jobs ADD COLUMN commander_effort_override TEXT;
