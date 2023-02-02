# Rockpass

A small and ultrasecure [LessPass][lesspass] database server written in
[Rust][rust].

[rust]: https://www.rust-lang.org/

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
[hub]: https://hub.docker.com/r/ogarcia/rockpass

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
| `ROCKPASS_DATABASES` | Database location | {rockpass={url="/var/lib/rockpass/rockpass.sqlite"}} |
| `ROCKPASS_ADDRESS` | Listen address | 0.0.0.0 |
| `ROCKPASS_PORT` | Listen port | 8000 |
| `ROCKPASS_REGISTRATION_ENABLED` | Enable or disable the ability to register new users | true |
| `ROCKPASS_ACCESS_TOKEN_LIFETIME` | Time, in seconds, that the access token is valid | 3600 (1 hour) |
| `ROCKPASS_REFRESH_TOKEN_LIFETIME` | Time, in seconds, that the refresh token is valid | 2592000 (30 days) |
| `ROCKPASS_LOG_LEVEL` | Log level | critical |

### From source

#### Installing Rust

Rockpass build has been tested with current Rust stable release version. You
can install Rust from your distribution package or use [`rustup`][rustup].
```
rustup default stable
```

If you prefer, you can use the stable version only for install Rockpass (you
must clone the repository first).
```
rustup override set stable
```

[rustup]: https://rustup.rs/

#### Installing Rockpass

To build Rockpass binary simply execute the following commands.
```sh
git clone https://github.com/ogarcia/rockpass.git
cd rockpass
cargo build --release
```

After build the binary is located in `target/release/rockpass`.

## Configuration

How Rockpass uses [Rocket][rocket] certain configuration parameters are
compatible with each other. You can look at the [Rocket configuration
documentation][rcdoc] to see what the basic parameters are. In any case,
a fully commented `rockpass.toml.example` is provided and the most important
parameters are detailed below.

| Setting | Use | Default value |
| --- | --- | --- |
| `address` | Listen address | 127.0.0.1 |
| `port` | Listen port | 8000 |
| `registration_enabled` | Enable or disable the ability to register new users | true |
| `access_token_lifetime` | Time, in seconds, that the access token is valid | 3600 (1 hour) |
| `refresh_token_lifetime` | Time, in seconds, that the refresh token is valid | 2592000 (30 days) |
| `databases` | SQLite database location (see below) | {rockpass={url=":memory:"}} |

The database configuration can be detailed in three options.

Option One.
```
databases = { rockpass = { url = "/location/of/rockpass.sqlite" } }
```

Option Two.
```
[release.databases]
rockpass = { url = "/location/of/rockpass.sqlite" }
```

Option Three.
```
[release.databases.rockpass]
url = "/location/of/rockpass.sqlite"
```

If you don't want use a config file you can use environment variables. For
example to disable registration and listen in 8080.
```
export ROCKPASS_PORT=8080
export ROCKPASS_REGISTRATION_ENABLED=false
export ROCKPASS_DATABASES='{ rockpass = { url = "/location/of/rockpass.sqlite" } }'
rockpass
```

You can run Rockpass without any configuration. By default it creates an
in-memory DB that is deleted once the process stops, this is useful for
testing purposes.
```
rockpass
```

The latter is exactly the same as running Rockpass by setting the `url` key
to `:memory:`.
```
ROCKPASS_DATABASES='{rockpass = { url = ":memory:" }}' rockpass
```

[rocket]: https://rocket.rs
[rcdoc]: https://rocket.rs/v0.5-rc/guide/configuration/#configuration

## Usage

Rockpass is an API server for LessPass so it does not expose any interface.
You can use any of the [existing LessPass applications][lpapps] (plugins,
mobile...) to connect against the server or my own
[lesspass-client][lesspass-client] command line client implementation.

### Example of use with lesspass-client

Let's see an example of use with `lesspass-client`. First we start
`rockpass` without options or configuration to work directly in memory (it
is an example). When we feel comfortable we can start Rockpass with its
final configuration.
```
rockpass
```

The first thing we need to be able to connect is to create a user. For this
user to be compatible with the official LessPass applications we must
encrypt their password as LessPass does. We can do this with the
lesspass-client itself as follows.
```sh
$ lesspass-client -m Login_Password password build lesspass.com login@mail.com
|O}'bU/sW*7Dfw->
```

What we have done is encrypt a password `Login_Password` for the user
`login@mail.com`, this has resulted in the encrypted password
`|O}'bU/sW*7Dfw->` which is the one we must use to create the user.

We now create the user as follows.
```sh
$ lesspass-client \
  -s http://127.0.0.1:8000 \
  user create "login@mail.com" "|O}'bU/sW*7Dfw->"
New user created successfully
```

From here we can connect against the API either with the client applications
or with the lesspass-client itself. If we do it with any of the client
applications (for example the [Firefox plugin][ffplugin]), we will use as
username `login@mail.com` and password `Login_Password` since the client
application itself will be responsible for encrypting it. If, on the other
hand, we do it with lesspass-client, the username will be the same but we
must use the encrypted password as detailed below.
```sh
# Add a new password profile
$ lesspass-client \
  -s http://127.0.0.1:8000 \
  -u "login@mail.com" \
  -p "|O}'bU/sW*7Dfw->" \
  password add example.com login@mail.com
New password created successfully

# List profiles stored on the server
$ lesspass-client \
  -s http://127.0.0.1:8000 \
  -u "login@mail.com" \
  -p "|O}'bU/sW*7Dfw->" \
  password list
example.com

# Show a profile
$ lesspass-client \
  -s http://127.0.0.1:8000 \
  -u "login@mail.com" \
  -p "|O}'bU/sW*7Dfw->" \
  password show example.com
ID: 1
Site: example.com
Login: login@mail.com
Lowercase: true
Uppercase: true
Symbols: true
Numbers: true
Length: 16
Couter: 1

# Encrypt a new password using a master password and the created profile
# Option One (Master password as environment variable)
$ export LESSPASS_MASTERPASS="very difficult master password"
$ lesspass-client \
  -s http://127.0.0.1:8000 \
  -u "login@mail.com" \
  -p "|O}'bU/sW*7Dfw->" \
  password show -p example.com
X?%x0O=yn&n4cWGU
# Option Two (Master password as argument)
$ lesspass-client \
  -s http://127.0.0.1:8000 \
  -u "login@mail.com" \
  -p "|O}'bU/sW*7Dfw->" \
  -m "very difficult master password as argument" \
  password show -p example.com
:~xd`ZtYvS1/8I2+
# The passwords are different in each example because we have changed the
# master password.
```

All of the above is just an example, lesspass-client is a complete client so
it is possible to perform multiple operations on the LessPass API. See the
command help for more information.

[lpapps]: https://www.lesspass.com/#supported-platforms
[ffplugin]: https://addons.mozilla.org/en-US/firefox/addon/lesspass/

## Known limitations

### Password reset

With the premise in mind of keeping the code simple (remember that it is for
personal use so KISS), Rockpass has not implemented any password reset API.
However, if any user does not remember their password, you can reset it by
following the procedure below.

#### Encrypt your password

LessPass encrypts the passwords in the client before sending them, so we
must perform the same procedure since for each user the encrypted password
is different. My advice is to use a temporary easy password (in my examples
I will use `123456`) and then the user will change it to something more
secure.

To encrypt the password we can use [LessPass][lesspass] itself or my
[lesspass-client][lesspass-client]. The parameters to use are the following.

- Site: `lesspass.com`
- Login: The email address you use to authenticate. I will use
  `rockpass@example.com` in this example.
- Master password: The new password you want to use. In my case as I said
  before `123456`.
- Options: Default. This means all options checked, size 16 and counter 1.

If we want to encrypt the password with lesspass-client the command would
be.
```sh
$ lesspass-client -m 123456 password build lesspass.com rockpass@example.com
-!?}5Nx9E\%svt07
```

Regardless of how we encrypt the password the result is that our encrypted
password is `-!?}5Nx9E\%svt07`. Obviously this is only valid if the user
name is `rockpass@example.com`, if you use another user name it will
generate a different encrypted password.

#### Encrypt your password (again)

Even if the password is encrypted by the client, Rockpass encrypts it again
with Bcrypt hash, this is because in the client options you can select that
the password is not encrypted and to increase security.

To encrypt the password with [Bcrypt][bcrypt] the easiest way is to use
a _web application_ that does it, for example [Bcrypt-Generator][bg] or
[Bcrypt-Online][bo]. When encrypting with Bcrypt you have to set a cost
factor, Rockpass uses a factor of 10, but it is OK to use a different
factor.

Note that each time you encrypt with Bcrypt a new string is generated, this
is normal since Bcrypt generates a different _salt_ each time. In my case
the password generated from `-!?}5Nx9E\%svt07` is
`$2y$10$EYeEDnNN/dIQOkJtU9VQAe2fGXhncHbsdZnTdmrs6JMaceKqLg.X6`.

[bcrypt]: https://en.wikipedia.org/wiki/Bcrypt
[bg]: https://bcrypt-generator.com/
[bo]: https://bcrypt.online/

#### Update the password in the database

1. _Connect_ to database.
   ```sh
   sqlite3 /location/of/rockpass.sqlite
   ```
2. Update password to the value `123456` for the user `rockpass@example.com`.
   ```sql
   UPDATE users SET
    password='$2y$10$EYeEDnNN/dIQOkJtU9VQAe2fGXhncHbsdZnTdmrs6JMaceKqLg.X6'
    WHERE email = 'rockpass@example.com';
   ```

Now it would be possible to authenticate with the user
`rockpass@example.com` with the password `123456` and modify it with the
application. Don't forget to do it!

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

[lesspass]: https://lesspass.com/
[lesspass-client]: https://github.com/ogarcia/lesspass-client
