FROM rust:1.80.0 AS app-builder
ARG BUILD_BRANCH=main
RUN --mount=type=secret,id=pat export PAT=$(cat /run/secrets/pat) && \
            git clone https://$PAT@github.com/fatlabsxyz/miden-cli-wraper.git && \
            cd miden-cli-wraper && \
            git checkout $BUILD_BRANCH
RUN cd miden-cli-wraper && cargo build --release
RUN cd miden-cli-wraper && cargo build

FROM rust:1.80.0 AS cli-builder
RUN --mount=type=secret,id=pat export PAT=$(cat /run/secrets/pat) && \
            git clone https://$PAT@github.com/fatlabsxyz/miden-cli-wraper.git && \
            cd miden-cli-wraper && \
            cargo build --release
RUN cargo install --root /miden-cli miden-cli --features concurrent,testing

FROM debian:bookworm-slim AS runner
RUN apt update && apt install -y libsqlite3-0
WORKDIR /app
COPY --from=app-builder /miden-cli-wraper/target/release/wraper-cli /app/
COPY --from=app-builder /miden-cli-wraper/target/debug/wraper-cli /app/wraper-cli-debug
COPY --from=cli-builder /miden-cli/bin/miden /app/
COPY Rocket.toml /app/
ENV MIDEN_CLIENT_CLI="/app/miden"
ENV USERS_DB_DIR="/app/db/users"
ENV USERNAME_DB_DIR="/app/db/usernames"

CMD ["/app/wraper-cli"]
