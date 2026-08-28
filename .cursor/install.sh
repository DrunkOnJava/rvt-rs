#!/usr/bin/env bash
#
# Idempotent bootstrap for the rvt-rs development environment.
#
# Sets up all three buildable components:
#   1. Rust core library + CLI binaries (needs Rust >= 1.85 for edition 2024)
#   2. Python bindings (maturin + pyo3, built into a local venv)
#   3. WebAssembly viewer (wasm-pack -> viewer/pkg + Vite/npm)
#
# Safe to re-run: every step converges to the same state and skips work that
# is already done. Runs after the repository is checked out.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

log() { printf '\n=== %s ===\n' "$1"; }

# --- 1. System packages ----------------------------------------------------
# python3-venv: stdlib venv is not shipped by default on Debian/Ubuntu.
# wabt:         provides wasm-objdump for the viewer's network-import audit.
log "System packages (python3-venv, wabt)"
if command -v apt-get >/dev/null 2>&1; then
  NEED_APT=()
  dpkg -s python3-venv >/dev/null 2>&1 || NEED_APT+=(python3-venv)
  command -v wasm-objdump >/dev/null 2>&1 || NEED_APT+=(wabt)
  if [ "${#NEED_APT[@]}" -gt 0 ]; then
    sudo apt-get update -y
    sudo apt-get install -y "${NEED_APT[@]}"
  else
    echo "already present"
  fi
fi

# --- 2. Rust toolchain -----------------------------------------------------
# The crate is edition 2024 with rust-version = 1.85; install and default to
# the stable channel and make sure the wasm target + fmt/clippy are present.
log "Rust toolchain (stable + wasm32 target)"
rustup toolchain install stable --profile minimal --no-self-update
rustup default stable
rustup component add rustfmt clippy
rustup target add wasm32-unknown-unknown
rustc --version

# --- 3. wasm-pack ----------------------------------------------------------
# Pin to the same version CI uses (deploy-viewer.yml) so the WASM output
# is byte-for-byte comparable and doesn't drift between local and CI. Fall
# back to the upstream installer (latest) only if the pinned download fails,
# so a GitHub hiccup never blocks setup entirely.
WASM_PACK_VERSION="v0.14.0"
log "wasm-pack ${WASM_PACK_VERSION}"
if command -v wasm-pack >/dev/null 2>&1 && \
   wasm-pack --version 2>/dev/null | grep -q "${WASM_PACK_VERSION#v}"; then
  echo "already installed"
else
  arch="$(uname -m)"
  pkg="wasm-pack-${WASM_PACK_VERSION}-${arch}-unknown-linux-musl"
  url="https://github.com/rustwasm/wasm-pack/releases/download/${WASM_PACK_VERSION}/${pkg}.tar.gz"
  tmp="$(mktemp -d)"
  if curl -fsSL "$url" -o "$tmp/wasm-pack.tar.gz" && tar -xzf "$tmp/wasm-pack.tar.gz" -C "$tmp"; then
    dest_dir="$(dirname "$(command -v cargo)")"
    if [ -w "$dest_dir" ]; then
      install -m 0755 "$tmp/$pkg/wasm-pack" "$dest_dir/wasm-pack"
    else
      sudo install -m 0755 "$tmp/$pkg/wasm-pack" "$dest_dir/wasm-pack"
    fi
    rm -rf "$tmp"
  else
    echo "pinned download failed — falling back to upstream installer"
    curl -sSf https://rustwasm.github.io/wasm-pack/installer/init.sh | sh
  fi
fi
wasm-pack --version

# --- 4. Build the Rust workspace (library + CLI binaries) ------------------
log "cargo build --release"
cargo build --release

# --- 5. Python bindings ----------------------------------------------------
log "Python venv + maturin develop"
if [ ! -d .venv ]; then
  python3 -m venv .venv
fi
# shellcheck disable=SC1091
. .venv/bin/activate
python -m pip install --upgrade pip maturin pytest
maturin develop --manifest-path rvt-py/Cargo.toml
python -c "import rvt; print('rvt (python):', rvt.__version__)"
deactivate

# --- 6. WebAssembly viewer -------------------------------------------------
log "Build WASM package into viewer/pkg"
wasm-pack build --target web -- --features wasm --no-default-features
rm -rf viewer/pkg
mv pkg viewer/pkg

log "Viewer npm dependencies + type check + build"
(
  cd viewer
  npm ci
  npm run typecheck
  npm run build
)

# --- 7. Self-verification smoke -------------------------------------------
# Prove the freshly built artifacts actually work before this snapshot is
# blessed. A broken build fails here (and fails the environment build) rather
# than surfacing later inside an agent session.
log "Smoke test (fixture -> rvt-inspect, wasm import audit)"
SMOKE_DIR="$(mktemp -d)"
./target/release/gen-fixture smoke \
  --classes Wall,Level,Project --element-count 5 --year 2024 \
  --output "$SMOKE_DIR/smoke.rvt"
./target/release/rvt-inspect "$SMOKE_DIR/smoke.rvt" >/dev/null
echo "  rvt-inspect: OK"
rm -rf "$SMOKE_DIR"

# VW1-21 privacy invariant: the viewer WASM must import no network primitives.
if command -v wasm-objdump >/dev/null 2>&1; then
  if wasm-objdump -j Import -x viewer/pkg/rvt_bg.wasm | \
       grep -iE '"(fetch|XMLHttpRequest|WebSocket|EventSource|sendBeacon)"'; then
    echo "ERROR: VW1-21 violation — wasm imports a network primitive" >&2
    exit 1
  fi
  echo "  wasm network-import audit: PASS"
fi

log "rvt-rs environment ready"
