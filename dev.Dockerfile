FROM docker.io/rust:1-alpine3.22 AS build

COPY . .

RUN apk add openssl-dev musl-dev openssl-libs-static
RUN cargo build -r
RUN objcopy --compress-debug-sections target/release/server ./server

FROM docker.io/alpine:3.22.0

COPY --from=build /server ./server
ADD https://alts-alt.online ./static/not_found.html
ADD https://alts-alt.online/main.js ./static/main.js
ADD https://alts-alt.online/favicon.ico ./static/favicon.ico
COPY ./Rocket.toml ./Rocket.toml

EXPOSE 8000
CMD ["./server"]
