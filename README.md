# Rockpass

A small and ultrasecure [Lesspass][1] database server written in [Rust][2].

[1]: https://lesspass.com/
[2]: https://www.rust-lang.org/

## Installation

### Installing Rust

Rockpass is based in [Rocket][3] so you need to use a nightly version of
Rust.
```
rustup default nightly
```

If you prefer, you can use the nightly version only for install Rockpass.
```
rustup override set nightly
```

[3]: https://rocket.rs/

### Installing Rockpass

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
comments and the interesting field are the following.

|         Setting        |                         Use                         |   Default value   |
|:----------------------:|:---------------------------------------------------:|:-----------------:|
| `registration_enabled` | Enable or disable the ability to register new users | true              |
| `token_lifetime`       | Time, in seconds, that the login token is valid     | 2592000 (30 days) |
| `rockpass`             | SQLite database location (see below)                |                   |

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

Finally if you want to execute the server witout database (for testing) you
can configure `url` as `:memory:`.
```
ROCKET_DATABASES='{rockpass = { url = ":memory:" }}' rockpass
```

[4]: https://rocket.rs/v0.4/guide/configuration/

## Known limitations

For now, user deletion cannot be done via API. If you want to delete an user
you can do it with `sqlite` command. For example to delete user
_user@example.com_ and all of his/her passwords settings.

1. _Connect_ to database.
   ```sh
   sqlite3 /location/of/rockpass.sqlite
   ```
2. Detete user.
   ```sql
   PRAGMA foreign_keys = ON;
   DELETE FROM users WHERE email = 'user@example.com';
   ```
