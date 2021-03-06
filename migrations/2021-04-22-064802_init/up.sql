CREATE TABLE IF NOT EXISTS users (
  id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
  email TEXT NOT NULL UNIQUE,
  password TEXT NOT NULL,
  token TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS passwords (
  id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  login TEXT NOT NULL,
  site TEXT NOT NULL,
  uppercase BOOLEAN NOT NULL DEFAULT TRUE,
  symbols BOOLEAN NOT NULL DEFAULT TRUE,
  lowercase BOOLEAN NOT NULL DEFAULT TRUE,
  numbers BOOLEAN NOT NULL DEFAULT TRUE,
  counter INTEGER NOT NULL DEFAULT 1,
  version INTEGER NOT NULL DEFAULT 2,
  length INTEGER NOT NULL DEFAULT 16,
  created DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  modified DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX IF NOT EXISTS users_unique ON users (token);
CREATE UNIQUE INDEX IF NOT EXISTS passwords_unique ON passwords (user_id, login, site);
