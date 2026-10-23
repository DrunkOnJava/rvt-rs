#!/usr/bin/env bash
#
# Idempotent bootstrap for the rvt-rs development environment.
#
# Sets up all three buildable components:
#   1. Rust core library + CLI binaries (needs Rust >= 1.87, the MSRV)
#   2. Python bindings (maturin + pyo3, built into a local venv)
#   3. WebAssembly viewer (wasm-pack -> viewer/pkg + Vite/pnpm)
#
# Safe to re-run: every step converges to the same state and skips work that
# is already done. Runs after the repository is checked out.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"
BUILD_PROFILE="${RVT_BUILD_PROFILE:-release}"
case "$BUILD_PROFILE" in release|ci|dev) ;; *) echo "Unsupported build profile: $BUILD_PROFILE" >&2; exit 1 ;; esac
WASM_PROFILE="${RVT_WASM_PROFILE:-release}"
case "$WASM_PROFILE" in release|dev) ;; *) echo "Unsupported WASM profile: $WASM_PROFILE" >&2; exit 1 ;; esac
BIN_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}/$BUILD_PROFILE"
[ "$BUILD_PROFILE" != dev ] || BIN_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}/debug"

log() { printf '\n=== %s ===\n' "$1"; }

# --- 1. System packages ----------------------------------------------------
# python3-venv: stdlib venv is not shipped by default on Debian/Ubuntu.
log "System packages (python3-venv)"
if command -v apt-get >/dev/null 2>&1; then
  NEED_APT=()
  dpkg -s python3-venv >/dev/null 2>&1 || NEED_APT+=(python3-venv)
  if [ "${#NEED_APT[@]}" -gt 0 ]; then
    sudo apt-get update -y
    sudo apt-get install -y "${NEED_APT[@]}"
  else
    echo "already present"
  fi
fi

# --- 2. Rust toolchain -----------------------------------------------------
# The crate is edition 2024 with rust-version = 1.87; install and default to
# the stable channel and make sure the wasm target + fmt/clippy are present.
log "Rust toolchain (stable + wasm32 target)"
rustup toolchain install stable --profile minimal --no-self-update
rustup default stable
rustup component add rustfmt clippy
rustup target add wasm32-unknown-unknown
rustc --version

# --- 3. wasm-pack ----------------------------------------------------------
# Pin to the same version CI uses (deploy-viewer.yml) so the WASM output
# is byte-for-byte comparable and doesn't drift between local and CI. The
# download is checked against the SHA-256 GitHub publishes for the release
# asset; setup stops rather than run an installer it cannot check.
WASM_PACK_VERSION="v0.14.0"
log "wasm-pack ${WASM_PACK_VERSION}"
if command -v wasm-pack >/dev/null 2>&1 && \
   [ "$(wasm-pack --version)" = "wasm-pack ${WASM_PACK_VERSION#v}" ]; then
  echo "already installed"
else
  arch="$(uname -m)"
  case "$arch" in
    x86_64) sha256="278a8d668085821f4d1a637bd864f1713f872b0ae3a118c77562a308c0abfe8d" ;;
    aarch64) sha256="5941c7b05060440ff37ee50fe9009a408e63fa5ba607a3b0736f5a887ec5f2ca" ;;
    *) echo "no pinned wasm-pack ${WASM_PACK_VERSION} for ${arch}" >&2; exit 1 ;;
  esac
  pkg="wasm-pack-${WASM_PACK_VERSION}-${arch}-unknown-linux-musl"
  url="https://github.com/rustwasm/wasm-pack/releases/download/${WASM_PACK_VERSION}/${pkg}.tar.gz"
  tmp="$(mktemp -d)"
  if curl -fsSL "$url" -o "$tmp/wasm-pack.tar.gz" \
     && echo "${sha256}  $tmp/wasm-pack.tar.gz" | sha256sum -c - \
     && tar -xzf "$tmp/wasm-pack.tar.gz" -C "$tmp"; then
    dest_dir="$(dirname "$(command -v cargo)")"
    if [ -w "$dest_dir" ]; then
      install -m 0755 "$tmp/$pkg/wasm-pack" "$dest_dir/wasm-pack"
    else
      sudo install -m 0755 "$tmp/$pkg/wasm-pack" "$dest_dir/wasm-pack"
    fi
    rm -rf "$tmp"
  else
    echo "wasm-pack ${WASM_PACK_VERSION}: the download failed or its SHA-256 differs" >&2
    exit 1
  fi
fi
wasm-pack --version

# --- 4. Build the Rust workspace (library + CLI binaries) ------------------
log "cargo build --locked --profile $BUILD_PROFILE"
cargo build --locked --profile "$BUILD_PROFILE"

# --- 5. Python bindings ----------------------------------------------------
log "Python venv + maturin develop"
if [ ! -d .venv ]; then
  python3 -m venv .venv
fi
# shellcheck disable=SC1091
. .venv/bin/activate
python -m pip install --require-hashes -r tools/ci/requirements-dev.txt
maturin_args=()
if [ -n "${RVT_BUILD_PROFILE:-}" ]; then
  maturin_args+=(--profile "$BUILD_PROFILE")
fi
maturin develop --locked --manifest-path rvt-py/Cargo.toml "${maturin_args[@]}"
python -c "import rvt; print('rvt (python):', rvt.__version__)"
deactivate

# --- 6. WebAssembly viewer -------------------------------------------------
log "Build WASM package into viewer/pkg"
wasm-pack build rvt-wasm --"$WASM_PROFILE" --target web --out-dir ../viewer/pkg --out-name rvt -- --locked

log "Viewer pnpm dependencies + type check + build"
(
  cd viewer
  # pnpm at the version viewer/package.json pins in `packageManager`, through
  # Node's corepack, which checks the package it downloads.
  if ! command -v pnpm >/dev/null 2>&1; then
    corepack enable pnpm || {
      echo "corepack could not enable $(node -p 'require("./package.json").packageManager'); install it and rerun" >&2
      exit 1
    }
  fi
  pnpm install --frozen-lockfile
  pnpm run typecheck
  pnpm run build
)

# --- 7. Self-verification smoke -------------------------------------------
# Prove the freshly built artifacts actually work before this snapshot is
# blessed. A broken build fails here (and fails the environment build) rather
# than surfacing later inside an agent session.
log "Smoke test (fixture -> rvt-inspect, wasm import audit)"
SMOKE_DIR="$(mktemp -d)"
"$BIN_DIR/gen-fixture" smoke \
  --classes Wall,Level,Project --element-count 5 --year 2024 \
  --output "$SMOKE_DIR/smoke.rvt"
"$BIN_DIR/rvt-inspect" "$SMOKE_DIR/smoke.rvt" >/dev/null
echo "  rvt-inspect: OK"
rm -rf "$SMOKE_DIR"

# VW1-21 privacy invariant: the viewer WASM must import no network primitives.
node viewer/scripts/audit-wasm-imports.mjs

log "rvt-rs environment ready"
