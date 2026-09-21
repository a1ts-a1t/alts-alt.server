FROM alpine:3.22

RUN apk add --no-cache ca-certificates

RUN addgroup -S server && adduser -S server -G server

WORKDIR /app

# bring in kennel club data
ADD https://github.com/a1ts-a1t/kennel-club.git#:data ./kennel-club

COPY server ./server

RUN chown -R server:server /app
USER server

ARG SERVER_PORT
ARG WEBSITE_PORT
ARG WEBSITE_HOST

ENV SERVER_PORT=${SERVER_PORT}
ENV WEBSITE_PORT=${WEBSITE_PORT}
ENV WEBSITE_HOST=${WEBSITE_HOST}

EXPOSE ${SERVER_PORT}

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget -qO /dev/null "http://0.0.0.0:${SERVER_PORT:-8000}/api/ping" || exit 1

ENTRYPOINT ["./server"]
