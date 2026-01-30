#!/bin/bash

cleanup() {
    echo "Stopping services..."
    kill $FIBER_PID $SOCAT_PID 2>/dev/null
    wait $FIBER_PID $SOCAT_PID 2>/dev/null
    exit 0
}

trap cleanup SIGINT SIGTERM

socat TCP-LISTEN:10000,fork,bind=0.0.0.0,reuseaddr TCP:127.0.0.1:41716 &
SOCAT_PID=$!

RUST_LOG=info,fnn::watchtower::actor=warn FIBER_SECRET_KEY_PASSWORD=12345678 /fnn -c /config.yml -d . &
FIBER_PID=$!

wait $FIBER_PID $SOCAT_PID
