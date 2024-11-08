FROM rust:1.80.0 AS app-builder
ARG BUILD_BRANCH=master

# Clone the public repository
RUN git clone https://github.com/casiojapi/miden-api.git && \
    cd miden-api && \
    git checkout $BUILD_BRANCH
RUN cd miden-api && cargo build --release
RUN cd miden-api && cargo build

FROM rust:1.80.0 AS cli-builder
RUN git clone https://github.com/casiojapi/miden-api.git && \
    cd miden-api && \
    cargo build --release
RUN cargo install --root /miden-cli miden-cli --features concurrent,testing

FROM debian:bookworm-slim AS runner
RUN apt update && apt install -y libsqlite3-0
WORKDIR /app
COPY --from=app-builder /miden-api/target/release/wraper-cli /app/
COPY --from=app-builder /miden-api/target/debug/wraper-cli /app/wraper-cli-debug
COPY --from=cli-builder /miden-cli/bin/miden /app/
COPY Rocket.toml /app/

# Environment variables
ENV MIDEN_CLIENT_CLI="/app/miden"
ENV USERS_DB_DIR="/app/db/users"
ENV USERNAME_DB_DIR="/app/db/usernames"

EXPOSE 8000
CMD ["/app/wraper-cli"]
