# alts-alt.server

server code and ci for [alts-alt.online](https://alts-alt.online). 

## dev stuff

use the `dev.Dockerfile` to run a local instance of the server. it depends on [the website](https://alts-alt.online) being up, but if that's down, there are bigger fish to fry

assuming you have docker on your system, run the following

```sh
sudo docker compose -f compose.dev.yml -d up
```

