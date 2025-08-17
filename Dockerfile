FROM rust:latest as builder
WORKDIR /usr/src/pageshelf
COPY . .
RUN cargo install --path .

FROM rust:slim
RUN apt-get update & apt-get install -y extra-runtime-dependencies & rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/pageshelf /usr/local/bin/pageshelf
CMD ["pageshelf"]