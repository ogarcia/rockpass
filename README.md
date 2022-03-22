# Rockpass

A small and ultrasecure [Lesspass][1] database server written in [Rust][2].

[1]: https://lesspass.com/
[2]: https://www.rust-lang.org/

## Installation

### From binary

Simply download latest release from [releases page][releases]. You can use
[systemd unit][unit] from [Arch Linux package][package] to run it.

[releases]: https://github.com/ogarcia/rockpass/releases
[unit]: https://aur.archlinux.org/cgit/aur.git/tree/rockpass.service?h=rockpass
[package]: https://aur.archlinux.org/packages/rockpass

### With Docker

A docker image of Rockpass can be downloaded from [here][ghcr] or from
[Docker Hub][hub].

To run it, simply exec.
```
docker run -t -d \
  --name=rockpass \
  -p 8000:8000 \
  ogarcia/rockpass
```

This start Rockpass and publish the port to host.

Warning: this is a basic run, all data will be destroyed after container
stop and rm.

[ghcr]: https://github.com/users/ogarcia/packages/container/package/rockpass
[hub]: https://hub.docker.com/repository/docker/ogarcia/rockpass

#### Persist data using a Docker volume

Rockpass Docker image uses a volume `/var/lib/rockpass` to store sqlite
database. You can exec the following to mount it in your host as persistent
storage.
```
docker run -t -d \
  --name=rockpass \
  -p 8000:8000 \
  -v /my/rockpass:/var/lib/rockpass \
  ogarcia/rockpass
```

Take note that you must create before the directory `/my/rockpass` and set
ownership to UID/GID 100.
```
mkdir -p /my/rockpass
chown -R 100:100 /my/rockpass
```

#### Docker environment variables

| Variable | Used for | Default value |
| --- | --- | --- |
| `ROCKET_DATABASES` | Database location | {rockpass = { url = \"/var/lib/rockpass/rockpass.sqlite\" }} |
| `ROCKET_ADDRESS` | Listen address | 0.0.0.0 |
| `ROCKET_PORT` | Listen port | 8000 |
| `ROCKET_REGISTRATION_ENABLED` | Enable or disable the ability to register new users | true |
| `ROCKET_ACCESS_TOKEN_LIFETIME` | Time, in seconds, that the access token is valid | 3600 (1 hour) |
| `ROCKET_REFRESH_TOKEN_LIFETIME` | Time, in seconds, that the refresh token is valid | 2592000 (30 days) |
| `ROCKET_LOG_LEVEL` | Log level | normal |

### From source

#### Installing Rust

Rockpass is based in [Rocket][3] v0.5 so you can use stable version of Rust.
```
rustup default stable
```

If you prefer, you can use the stable version only for install Rockpass.
```
rustup override set stable
```

[3]: https://rocket.rs/

#### Installing Rockpass

To build Rockpass binary simply execute the following commands.
```sh
git clone https://github.com/ogarcia/rockpass.git
cd rockpass
cargo build --release
```

After build the binary is located in `target/release/rockpass`.

## Configuration

Since Rockpass is based in Rocket, the config is same that is detailed in
[Rocket documentation][4]. Anyway a `Rocket.toml.example` is provided with
comments and the interesting fields are the following.

| Setting | Use | Default value |
| --- | --- | --- |
| `registration_enabled` | Enable or disable the ability to register new users | true |
| `access_token_lifetime` | Time, in seconds, that the access token is valid | 3600 (1 hour) |
| `refresh_token_lifetime` | Time, in seconds, that the refresh token is valid | 2592000 (30 days) |
| `rockpass` | SQLite database location (see below) | |

The database configuration can be detailed in two options.

Option One.
```
[global.databases]
rockpass = { url = "/location/of/rockpass.sqlite" }
```

Option Two.
```
[global.databases.rockpass]
url = "/location/of/rockpass.sqlite"
```

If you don't want use a config file you can use environment variables. For
example to disable registration and listen in 8080.
```
export ROCKET_PORT=8080
export ROCKET_REGISTRATION_ENABLED=false
export ROCKET_DATABASES='{rockpass = { url = "/location/of/rockpass.sqlite" }}'
rockpass
```

Finally if you want to execute the server without database (for testing) you
can configure `url` as `:memory:`.
```
ROCKET_DATABASES='{rockpass = { url = ":memory:" }}' rockpass
```

[4]: https://rocket.rs/v0.4/guide/configuration/

## Known limitations

### Password reset

With the premise in mind of keeping the code simple (remember that it is for
personal use), Rockpass has not implemented any password reset API. But if
any user does not remember their password you can reset it to a default
value. For example, to reset the user _user@example.com_ password.

1. _Connect_ to database.
  ```sh
  sqlite3 /location/of/rockpass.sqlite
  ```
2. Update password to a default `123456` value.
   ```sql
   UPDATE users SET
    password='$2b$10$jJXcsuftQI9PwF8eqZo4/ObfBGbc.nhLBpV49fC4qeLLeh/Uz0YzW'
   WHERE email = 'user@example.com';
   ```

Now you can login with your username `user@example.com` and `123456` as
password. Don't forget to change it after login!

### Delete an user

For now, user deletion cannot be done via API. If you want to delete an user
you can do it with `sqlite` command. For example to delete user
_user@example.com_ and all of his/her passwords settings.

1. _Connect_ to database.
   ```sh
   sqlite3 /location/of/rockpass.sqlite
   ```
2. Delete user.
   ```sql
   PRAGMA foreign_keys = ON;
   DELETE FROM users WHERE email = 'user@example.com';
   ```
