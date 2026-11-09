-- Each time the person switched the Operator's master switch (`features.operator`) on or off, so the
-- Log can show a line where rulings before it are older. One row per change of value, never per save.
-- `n` orders them, since two moves can share a clock reading. Additive.
CREATE TABLE operator_switches (
    n          INTEGER PRIMARY KEY AUTOINCREMENT,
    turned_on  INTEGER NOT NULL,
    at         TEXT NOT NULL
);
