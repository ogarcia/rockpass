DROP INDEX IF EXISTS passwords_unique;

CREATE TABLE IF NOT EXISTS users_migration (
  id INTEGER NOT NULL PRIMARY KEY,
  email TEXT NOT NULL UNIQUE,
  password TEXT NOT NULL
);
INSERT INTO users_migration (
  id,
  email,
  password
) SELECT id, email, password FROM users;
DROP TABLE users;
ALTER TABLE users_migration RENAME TO users;

CREATE TABLE IF NOT EXISTS tokens_migration (
  id INTEGER NOT NULL PRIMARY KEY,
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  token TEXT NOT NULL UNIQUE,
  created DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  modified DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
INSERT INTO tokens_migration (
  id,
  user_id,
  token,
  created,
  modified
) SELECT id, user_id, token, created, modified FROM tokens;
DROP TABLE tokens;
ALTER TABLE tokens_migration RENAME TO tokens;

CREATE TABLE IF NOT EXISTS passwords_migration (
  id INTEGER NOT NULL PRIMARY KEY,
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
INSERT INTO passwords_migration (
  id,
  user_id,
  login,
  site,
  uppercase,
  symbols,
  lowercase,
  numbers,
  counter,
  version,
  length,
  created,
  modified
) SELECT id, user_id, login, site, uppercase, symbols, lowercase, numbers, counter, version, length, created, modified FROM passwords;
DROP TABLE passwords;
ALTER TABLE passwords_migration RENAME TO passwords;

CREATE UNIQUE INDEX IF NOT EXISTS passwords_unique ON passwords (user_id, login, site);
