# Builder stage - compile the fiber binary
FROM debian:12 AS builder

RUN apt-get update && \
    apt-get install -y git curl gcc g++ make libclang-dev && \
    rm -rf /var/lib/apt/lists/*

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y

RUN cd /root && git clone --recursive https://github.com/Officeyutong/fiber && \
    cd fiber && git checkout fiber-demo-support

RUN cd /root/fiber && /root/.cargo/bin/cargo build --release

# Runtime stage - slim image with only runtime dependencies
FROM debian:12-slim

RUN apt-get update && \
    apt-get install -y curl expect jq socat openssl && \
    rm -rf /var/lib/apt/lists/*

ARG NODE_NAME
RUN test -n "${NODE_NAME}" || (echo "NODE_NAME build arg must be set" && exit 1)

COPY --from=builder /root/fiber/target/release/fnn /fnn

COPY config-${NODE_NAME}.yml /config.yml
COPY dev.toml /
COPY contracts /contracts

RUN mkdir /ckb
COPY ckb-keys/${NODE_NAME}-key /ckb/key

RUN mkdir /fiber
COPY fiber-keys/${NODE_NAME}-key /fiber/sk
RUN chmod 400 /fiber/sk

COPY start.sh /start.sh
RUN chmod +x /start.sh

ENTRYPOINT ["/start.sh"]
