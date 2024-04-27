# ggxdocker/cosmos:v1
FROM golang:1.22-bookworm

WORKDIR /opt
RUN apt update \
    && apt install -y git make \
    && git clone https://github.com/ignite/cli ignite --branch v0.25.2 \
    && cd ignite \
    && make install

# split these RUNs intentionally to not rebuild `ignite` on every change...
RUN git clone https://github.com/ibc-test/oct-planet.git \
    && cd oct-planet \
    && git checkout 76836e611a37ec435891a9d848e2d9e19a483f34

WORKDIR /opt/oct-planet

# Command:
# ignite chain serve -f -v -c earth.yml
