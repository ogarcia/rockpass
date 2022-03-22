#! /bin/sh
#
# build.sh
# Copyright (C) 2021 Óscar García Amor <ogarcia@connectical.com>
#
# Distributed under terms of the GNU GPLv3 license.
#

# upgrade
apk -U --no-progress upgrade

# install build deps
apk --no-progress add build-base curl openssl openssl-dev sqlite-dev
curl https://sh.rustup.rs -sSf | sh -s -- -q -y --default-toolchain 1.59.0

# build rockpass
cd /rockpass/src
source $HOME/.cargo/env
RUSTFLAGS="-C target-feature=-crt-static" cargo build --release --locked --all-features

# package rockpass
install -Dm755 "target/release/rockpass" \
  "/rockpass/pkg/bin/rockpass"

# create rockpass user
adduser -S -D -H -h /var/lib/rockpass -s /sbin/nologin -G users \
  -g rockpass rockpass
install -d -m0755 "/rockpass/pkg/etc"
install -m644 "/etc/passwd" "/rockpass/pkg/etc/passwd"
install -m644 "/etc/group" "/rockpass/pkg/etc/group"
install -m640 -gshadow "/etc/shadow" "/rockpass/pkg/etc/shadow"
