# Build stage
FROM rust:latest AS builder

WORKDIR /usr/src/pageshelf

COPY . .
RUN cargo install --path . --locked --root /usr/local --profile release

# Runtime stage (Alpine)
FROM alpine:latest AS runtime

# Install minimal runtime dependencies
RUN apk add --no-cache \
    libssl3 \
    ca-certificates

COPY --from=builder /usr/local/bin/pageshelf /usr/local/bin/pageshelf

CMD ["pageshelf"]
