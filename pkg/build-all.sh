#!/usr/bin/env bash

set -e
pushd ../wasm
./build.sh
pushd ./demo
npm install
NODE_ENV=production npm run build
popd
popd
cargo build --release --bin pkg
