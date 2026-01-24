FROM rust:1.93-alpine AS builder

RUN apk add --no-cache musl-dev build-base python3

WORKDIR /b4n

COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:3.22 AS runner

RUN apk add --no-cache ca-certificates
RUN addgroup -S b4ngroup && adduser -S b4nuser -G b4ngroup

COPY --from=builder /b4n/target/x86_64-unknown-linux-musl/release/b4n /usr/local/bin/b4n
COPY ./assets/themes /home/b4nuser/.b4n/themes/
RUN chmod +x /usr/local/bin/b4n \
    && chown b4nuser:b4ngroup /usr/local/bin/b4n \
    && chown -R b4nuser:b4ngroup /home/b4nuser/.b4n

USER b4nuser

ENTRYPOINT ["/usr/local/bin/b4n"]
