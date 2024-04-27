# ggxdocker/hermes:v1
FROM rust:1.77-buster

ENV DEBIAN_FRONTEND=noninteractive
WORKDIR /opt
RUN apt update && apt install -y git \
    && git clone -b polkadot-v0.9.43 https://github.com/ibc-test/hermes.git \
    && cd /opt/hermes \
    && cargo build --release -p ibc-relayer-cli \
    && cp target/release/hermes /usr/local/bin/hermes \
    && cargo clean \
    && chmod +x /usr/local/bin/hermes \
    && hermes --version

# separate RUN to facilitate caching
RUN cargo install --force --locked cargo-contract --version 3.2.0
RUN git clone https://github.com/baidang201/ibc.git \
    && cd ibc/ics20demo \
    && rustup install nightly \
    && rustup +nightly component add rust-src \
    && cargo +nightly contract build --release
# contract is at /opt/ibc/target/ink/my_psp37_wrapper.{contract,json,wasm}

WORKDIR /opt/hermes
