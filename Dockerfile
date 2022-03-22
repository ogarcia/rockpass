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
    ROCKET_ADDRESS="0.0.0.0" \
    ROCKET_PORT=8000 \
    ROCKET_REGISTRATION_ENABLED=true \
    ROCKET_ACCESS_TOKEN_LIFETIME=3600 \
    ROCKET_REFRESH_TOKEN_LIFETIME=2592000 \
    ROCKET_LOG_LEVEL=normal
VOLUME [ "/var/lib/rockpass" ]
USER rockpass
ENTRYPOINT [ "/bin/rockpass" ]
