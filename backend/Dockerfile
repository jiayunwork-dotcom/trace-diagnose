FROM rust:1.82-alpine AS builder

WORKDIR /app

RUN apk add --no-cache musl-dev openssl-dev perl make

COPY Cargo.toml ./
COPY .cargo ./.cargo
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release || true

COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/trace-diagnose-backend .
COPY migrations ./migrations

EXPOSE 8080

CMD ["./trace-diagnose-backend"]
