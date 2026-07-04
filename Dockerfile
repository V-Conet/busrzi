# Build from source
FROM rust:alpine AS builder

WORKDIR /src

COPY . /src

RUN cargo build --release


FROM alpine:3.24

WORKDIR /app

COPY --from=builder /src/target/release/busrzi /app/busrzi

COPY assets /app/assets

ENV PORT=8080

EXPOSE 8080

CMD ["/app/busrzi"]