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

ARG NODE_DIR
RUN test -n "${NODE_DIR}" || (echo "NODE_DIR build arg must be set" && exit 1)

COPY --from=builder /root/fiber/target/release/fnn /fnn

COPY ${NODE_DIR}/config.yml /config.yml
COPY ${NODE_DIR}/dev.toml /
COPY contracts /contracts

RUN mkdir /ckb
COPY ${NODE_DIR}/ckb/key /ckb/key

RUN mkdir /fiber
COPY ${NODE_DIR}/fiber/sk /fiber/sk
RUN chmod 400 /fiber/sk

COPY start.sh /start.sh
RUN chmod +x /start.sh

ENTRYPOINT ["/start.sh"]
