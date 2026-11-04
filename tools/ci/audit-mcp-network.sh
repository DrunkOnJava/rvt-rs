#!/usr/bin/env bash
# Fail when the MCP server's dependency tree could reach the network.
#
# rvt-rs-mcp speaks MCP over stdio only: it opens no socket and makes no
# network call, so what it ships must carry no HTTP client or server, no TLS
# stack and no WebSocket transport. This lists the crates `rvt-rs-mcp` links
# into the binary (normal edges, dev-dependencies left out: the end-to-end
# test's rmcp client is one) and fails on any of them. `rmcp` stays on its
# `server`, `macros`, `transport-io` and `schemars` features, none of which
# pulls one in; turning on `transport-streamable-http-server` or a `reqwest`
# feature would, and this is what fails then. The end-to-end test checks the
# running process as well: on Linux it holds no socket.
#
# Usage: tools/ci/audit-mcp-network.sh
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

tree="$(mktemp)"
trap 'rm -f "$tree"' EXIT
cargo tree -p rvt-rs-mcp -e normal --prefix none --locked > "$tree"

if ! grep -q '^rmcp v' "$tree"; then
    echo "error: cargo tree did not list rmcp, so this check read nothing" >&2
    exit 1
fi

banned='^(hyper|hyper-util|hyper-rustls|hyper-tls|reqwest|h2|h3|http|http-body|http-body-util|httparse|axum|tower|tower-service|tower-http|warp|actix-web|tide|rustls|rustls-pemfile|rustls-webpki|native-tls|openssl|openssl-sys|ring|ureq|curl|curl-sys|isahc|attohttpc|surf|tungstenite|tokio-tungstenite|tokio-rustls|tokio-native-tls|webpki-roots|webpki|security-framework|schannel|sse-stream|async-tungstenite|quinn|reqwest-middleware|socket2|mio) v'
if grep -E "$banned" "$tree"; then
    echo "error: rvt-rs-mcp's dependency tree carries an HTTP, TLS or WebSocket crate (above)." >&2
    echo "       The server is stdio only; see rvt-mcp/Cargo.toml." >&2
    exit 1
fi

# tokio's `net` feature is what pulls in sockets (mio, socket2): rmcp's
# `transport-io` turns it on, so the server serves stdin and stdout itself.
features="$(mktemp)"
trap 'rm -f "$tree" "$features"' EXIT
cargo tree -p rvt-rs-mcp -e normal,features -i tokio --locked > "$features"
if ! grep -q 'tokio feature "io-std"' "$features"; then
    echo "error: cargo tree did not list tokio's features, so this check read nothing" >&2
    exit 1
fi
if grep -E 'tokio feature "(net|process|signal)"' "$features"; then
    echo "error: tokio's net, process or signal feature is on in rvt-rs-mcp's tree (above); it opens no socket." >&2
    exit 1
fi

echo "rvt-rs-mcp: $(grep -c '' "$tree") crates in its tree, none of them an HTTP, TLS or WebSocket crate, and tokio has no net feature"
