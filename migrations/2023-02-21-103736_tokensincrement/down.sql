CREATE TABLE IF NOT EXISTS tokens_migration (
  id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  access_token TEXT NOT NULL UNIQUE,
  refresh_token TEXT NOT NULL UNIQUE,
  created DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  modified DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
INSERT INTO tokens_migration (
  id,
  user_id,
  access_token,
  refresh_token,
  created,
  modified
) SELECT id, user_id, access_token, refresh_token, created, modified FROM tokens;
DROP TABLE tokens;
ALTER TABLE tokens_migration RENAME TO tokens;
