#!/bin/bash
# Force direct connection to crates.io bypassing all proxies and mirrors

# Unset all proxy variables
unset ALL_PROXY HTTP_PROXY HTTPS_PROXY http_proxy https_proxy

# Set cargo to use direct crates.io
export CARGO_REGISTRY_DEFAULT=https://github.com/rust-lang/crates.io-index
export CARGO_HTTP_CHECK_REVOKE=false
export CARGO_HTTP_TIMEOUT=60
export CARGO_NET_GIT_FETCH_WITH_CLI=true
export CARGO_NET_RETRY=10

echo "Building with direct connection to crates.io..."

# Run make with clean environment for cargo
make "$@"
