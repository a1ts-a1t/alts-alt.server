FROM rust:latest AS builder
WORKDIR /usr/src/alts-alt_server
COPY . .
RUN cargo build -r

FROM ubuntu:latest
RUN useradd -s "/sbin/nologin" server-runner
USER server-runner
COPY --chown=444 ./build ./build
COPY --from=builder /usr/src/alts-alt_server/target/release/alts-alt_server /usr/local/bin/alts-alt_server
ENV PORT=8080
ENV STATIC_SERVER_FALLBACK_FILE="build/not_found.html"
EXPOSE 8080
CMD ["alts-alt_server"]

