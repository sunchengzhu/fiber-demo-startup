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

COPY --from=builder /root/fiber/target/release/fnn /fnn

COPY contracts /contracts

ARG NODE_NAME=bootnode
COPY nodes/${NODE_NAME}/ckb/key /ckb/key

COPY start.sh /start.sh
RUN chmod +x /start.sh

ENTRYPOINT ["/start.sh"]
