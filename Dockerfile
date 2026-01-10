ARG RUST_VERSION=1.91.1
ARG APP_NAME=bot
ARG LIB_NAME=scheduler

FROM rust:${RUST_VERSION}-slim AS build
ARG APP_NAME
ARG LIB_NAME
WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev

ENV SQLX_OFFLINE=true

ARG APP_PATH=./$APP_NAME

RUN --mount=type=bind,source=$APP_PATH/src,target=$APP_NAME/src \
    --mount=type=bind,source=$APP_PATH/Cargo.toml,target=$APP_NAME/Cargo.toml \
    --mount=type=bind,source=$LIB_NAME/migrations,target=$LIB_NAME/migrations \
    --mount=type=bind,source=.sqlx,target=.sqlx \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=./$LIB_NAME,target=$LIB_NAME \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
cargo build --locked --release && \
cp ./target/release/$APP_NAME /bin/$APP_NAME

COPY entrypoint.sh /app/entrypoint.sh


FROM debian:bookworm-slim AS final
ARG APP_NAME

RUN apt-get update && apt-get install -y \
    ca-certificates \
    openssl

COPY --from=build /app/entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod +x /usr/local/bin/entrypoint.sh

# Create a non-privileged user that the app will run under.
# See https://docs.docker.com/go/dockerfile-user-best-practices/
ARG UID=10001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser
USER appuser

COPY --from=build /bin/$APP_NAME /bin/

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]

# What the container should run when it is started.
CMD ["/bin/bot"]
