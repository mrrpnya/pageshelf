FROM docker.io/blackdex/rust-musl:x86_64-musl AS chef
USER root
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin pageshelf --target x86_64-unknown-linux-musl
RUN ls

# Runtime Stage
FROM alpine:latest AS runtime
RUN apk add --no-cache ca-certificates
RUN addgroup -S myuser && adduser -S myuser -G myuser
USER myuser
WORKDIR /
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/pageshelf .
CMD ["./pageshelf"]