ARG ALPINE_VERSION=3.13.5

FROM alpine:${ALPINE_VERSION} AS builder
COPY . /rockpass/src
RUN /rockpass/src/.github/docker.sh

FROM alpine:${ALPINE_VERSION}
RUN apk -U --no-progress upgrade && \
    apk --no-progress add libgcc sqlite-libs && \
    install -d -m0755 -o100 -g100 /var/lib/rockpass && \
    rm -f /var/cache/apk/*
COPY --from=builder /rockpass/pkg /
EXPOSE 8000
ENV ROCKET_DATABASES="{rockpass = { url = \"/var/lib/rockpass/rockpass.sqlite\" }}" \
    ROCKET_SECRET_KEY="fIdKuZfnI2oUJg4HMrKB7RTXxXS5B2Yw9D5RpOaKciI=" \
    ROCKET_ADDRESS="0.0.0.0" \
    ROCKET_PORT=8000 \
    ROCKET_REGISTRATION_ENABLED=true \
    ROCKET_TOKEN_LIFETIME=2592000 \
    ROCKET_LOG=normal
VOLUME [ "/var/lib/rockpass" ]
USER rockpass
ENTRYPOINT [ "/bin/rockpass" ]
