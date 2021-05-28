DROP TABLE IF EXISTS tokens;

CREATE TABLE IF NOT EXISTS users_migration (
  id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
  email TEXT NOT NULL UNIQUE,
  password TEXT NOT NULL,
  token TEXT NOT NULL UNIQUE
);
INSERT INTO users_migration (id, email, password, token) SELECT id, email, password, id FROM users;
DROP TABLE users;
ALTER TABLE users_migration RENAME TO users;

CREATE UNIQUE INDEX IF NOT EXISTS users_unique ON users (token);
