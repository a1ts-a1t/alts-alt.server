# alts-alt.server

server code and ci for [alts-alt.online](https://alts-alt.online). 

## dev stuff

use the `dev.Dockerfile` to run a local instance of the server. it depends on a local deploy of [alts-alt.website](https://github.com/a1ts-a1t/alts-alt.website) being up.

assuming you have docker on your system, run the following

```sh
docker build -f dev.Dockerfile -t server:dev .
docker run -p 8000:8000 server:dev
```

