ARG ALPINE_VERSION=latest

FROM alpine:${ALPINE_VERSION}
RUN adduser -S -D -H -h /var/lib/rockpass -s /sbin/nologin -G users -g rockpass rockpass && \
    apk -U --no-progress add libgcc sqlite-libs && \
    install -d -m0755 -o100 -g100 /var/lib/rockpass && \
    rm -f /var/cache/apk/*
COPY target/release/rockpass /bin/rockpass
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
