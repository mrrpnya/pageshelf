# Build stage
FROM rust:latest AS builder

WORKDIR /usr/src/pageshelf

COPY . .
RUN cargo install --path . --locked --root /usr/local --profile release

# Runtime stage
FROM debian:bookworm-slim AS runtime

# Install minimal runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/bin/pageshelf /usr/local/bin/pageshelf

CMD ["pageshelf"]