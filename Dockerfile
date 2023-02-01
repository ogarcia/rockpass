ARG ALPINE_VERSION=3.17.1

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
ENV ROCKPASS_DATABASES="{rockpass={url=\"/var/lib/rockpass/rockpass.sqlite\"}}" \
    ROCKPASS_ADDRESS="0.0.0.0" \
    ROCKPASS_PORT=8000 \
    ROCKPASS_REGISTRATION_ENABLED=true \
    ROCKPASS_ACCESS_TOKEN_LIFETIME=3600 \
    ROCKPASS_REFRESH_TOKEN_LIFETIME=2592000 \
    ROCKPASS_LOG_LEVEL=critical
VOLUME [ "/var/lib/rockpass" ]
USER rockpass
ENTRYPOINT [ "/bin/rockpass" ]
