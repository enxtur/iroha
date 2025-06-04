#!/bin/bash

MODE=$1

SAMPLE_TRX_HASH="47778CA463FD729710176637FB2B52E6C542070DE636D5877D6A43FF7319A0A1"

cargo build --release --bin iroha

if [ "$MODE" == "ping" ]; then
  target/release/iroha --config defaults/client.toml transaction ping --msg "Hello, world!"
elif [ "$MODE" == "trx" ]; then
  target/release/iroha --config defaults/client.toml transaction get --hash "$SAMPLE_TRX_HASH"
elif [ "$MODE" == "help" ]; then
  target/release/iroha --config defaults/client.toml --help
elif [ "$MODE" == "version" ]; then
  target/release/iroha --config defaults/client.toml version
elif [ "$MODE" == "test" ]; then
  target/release/iroha --version
fi