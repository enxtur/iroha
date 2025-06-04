#!/bin/bash

# Set environment variables
export CHAIN="00000000-0000-0000-0000-000000000000"
export PUBLIC_KEY="ed0120A98BAFB0663CE08D75EBD506FEC38A84E576A7C9B0897693ED4B04FD9EF2D18D"
export PRIVATE_KEY="802620A4DFC16789FBF9A588525E4AC7F791AC51B12AEE8919EACC03EB2FC31D32C692"
export P2P_PUBLIC_ADDRESS="localhost:1337"
export P2P_ADDRESS="0.0.0.0:1337"
export API_ADDRESS="0.0.0.0:8080"
export GENESIS_PUBLIC_KEY="ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4"
export GENESIS_PRIVATE_KEY="80262082B3BDE54AEBECA4146257DA0DE8D59D8E46D5FE34887DCD8072866792FCB3AD"
export GENESIS="/tmp/genesis.signed.scale"
export TOPOLOGY='["ed0120A98BAFB0663CE08D75EBD506FEC38A84E576A7C9B0897693ED4B04FD9EF2D18D"]'

MODE=$1

if [ -n "$MODE" ]; then
  if [ "$MODE" == "clean" ]; then
    rm -rf /tmp/genesis.json
    rm -rf /tmp/genesis.signed.scale
    exit 0
  elif [ "$MODE" == "init" ]; then
    mkdir -p .cache/wasmtime

    # Process genesis.json
    EXECUTOR_RELATIVE_PATH=$(jq -r '.executor' defaults/genesis.json)
    EXECUTOR_ABSOLUTE_PATH=$(realpath "defaults/$EXECUTOR_RELATIVE_PATH")
    WASM_DIR_RELATIVE_PATH=$(jq -r '.wasm_dir' defaults/genesis.json)
    WASM_DIR_ABSOLUTE_PATH=$(realpath "defaults/$WASM_DIR_RELATIVE_PATH")

    # Create modified genesis.json
    jq \
        --arg executor "$EXECUTOR_ABSOLUTE_PATH" \
        --arg wasm_dir "$WASM_DIR_ABSOLUTE_PATH" \
        --argjson topology "$TOPOLOGY" \
        '.executor = $executor | .wasm_dir = $wasm_dir | .topology = $topology' defaults/genesis.json \
        > /tmp/genesis.json

    # Sign genesis block
    cargo run --bin kagami genesis sign /tmp/genesis.json \
        --public-key "$GENESIS_PUBLIC_KEY" \
        --private-key "$GENESIS_PRIVATE_KEY" \
        --out-file "$GENESIS"
  else
    echo "Usage: $0 [init|clean]"
    echo "No argument: just start irohad"
    echo "init: prepare genesis and start irohad"
    echo "clean: clean up generated files and exit"
    exit 1
  fi
fi

# Start irohad
cargo run --bin irohad