-- Where each profile's shared home is or will be made on this Mac: the owner (the login or one of
-- its organisations) and the repository's name. `found` is set where the forge already had the
-- home, made on another Mac, and not set where a person chose it. A login with no row has the
-- default. Additive.
CREATE TABLE home_repositories (
    login TEXT NOT NULL PRIMARY KEY,
    owner TEXT NOT NULL,
    name  TEXT NOT NULL,
    found INTEGER NOT NULL DEFAULT 0
) STRICT;
