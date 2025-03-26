FROM ubuntu:latest
RUN useradd -s "/sbin/nologin" server-runner
USER server-runner
COPY --chown=444 ./build ./build
COPY --chown=744 ./alts-alt_server /usr/local/bin/server
ENV PORT=8080
ENV STATIC_SERVER_FALLBACK_FILE="build/not_found.html"
EXPOSE 8080
CMD ["server"]

