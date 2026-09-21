FROM rust:1-alpine3.22

RUN apk add --no-cache openssl-dev musl-dev openssl-libs-static

RUN addgroup -S server && adduser -S server -G server

WORKDIR /src

COPY . .
RUN cargo build -r

# bring in kennel club data
ADD https://github.com/a1ts-a1t/kennel-club.git#:data ./kennel-club

ARG SERVER_PORT=8000
ARG WEBSITE_PORT=4321

ENV SERVER_PORT=${SERVER_PORT}
ENV WEBSITE_PORT=${WEBSITE_PORT}
EXPOSE ${SERVER_PORT}

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget -qO /dev/null "http://0.0.0.0:${SERVER_PORT:-8000}/api/ping" || exit 1

USER server
CMD ["./target/release/server"]

