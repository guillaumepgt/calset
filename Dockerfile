FROM rust:1.89-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

FROM alpine:latest
COPY --from=builder /app/target/release/calset /usr/local/bin/calset
USER 1000:1000
CMD ["calset"]