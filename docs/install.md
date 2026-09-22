# Install Guide

This guide covers the supported ways to install or run rvt-rs on a fresh
machine. Use [user-guide.md](user-guide.md) after installation to decide which
workflow fits your file.

**Publication status (as of 2026-08-29, tree at `0.1.2`):**

| Channel | Status |
|---|---|
| **PyPI** (`rvt`) | **Published** — `pip install rvt` installs **0.1.2** |
| **crates.io** (`rvt`) | **Not published** — `cargo install rvt` will fail until a successful `cargo publish` |
| **docs.rs** (`rvt`) | **Not available** (404) until the crate exists on crates.io |
| **Prebuilt CLI binaries** (GitHub Releases) | **From the next tagged release** — the release workflow attaches archives for Linux, macOS and Windows; no release has shipped them yet |

Until a release carries binaries, build the Rust CLIs from source. See
[release-0.2.0-plan.md](release-0.2.0-plan.md) for the inspection-focused
alpha cut that aims to close the crates.io gap.

## Browser Viewer

No install is required for the hosted viewer:

1. Open <https://drunkonjava.github.io/rvt-rs/>.
2. Drop a `.rvt`, `.rfa`, `.rte`, or `.rft` file.
3. Confirm the File status panel before exporting.

The viewer is client-side only. It does not upload model bytes.

## Python Package From PyPI (available)

```bash
python -m venv .venv
. .venv/bin/activate
python -m pip install --upgrade pip
python -m pip install "rvt==0.1.2"
python -c "import rvt; print(rvt.__version__)"
```

Smoke test after installation:

```bash
python - <<'PY'
import json
import rvt

path = "path/to/model.rvt"
f = rvt.RevitFile(path)
print("version:", f.version)
print(json.loads(f.export_diagnostics_json())["confidence"]["level"])
PY
```

## Rust CLI From crates.io (not published yet)

The Cargo package name is `rvt`. **It is not on crates.io today**, so
commands like `cargo install rvt` / `cargo add rvt` will not resolve.
When a version is successfully published, the expected install path is:

```bash
cargo install rvt --locked
rvt-inspect --version
rvt-ifc --help
```

Until then, use [Build From Source](#build-from-source) below. After the
first crates.io publish, docs.rs should populate automatically at
<https://docs.rs/rvt> — verify with an HTTP 200 before linking it from
announcements.

## Prebuilt Binaries (GitHub Releases)

Every tagged release from the next one on attaches ready-to-run archives of
all the CLIs (`rvt-info`, `rvt-inspect`, `rvt-ifc`, …) to its page under
<https://github.com/DrunkOnJava/rvt-rs/releases>. No Rust toolchain is
needed.

| Platform | Archive |
|---|---|
| Linux x86_64 (static, any distribution) | `rvt-rs-<version>-x86_64-unknown-linux-musl.tar.gz` |
| Linux arm64 (static, any distribution) | `rvt-rs-<version>-aarch64-unknown-linux-musl.tar.gz` |
| macOS Apple silicon | `rvt-rs-<version>-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `rvt-rs-<version>-x86_64-apple-darwin.tar.gz` |
| Windows x86_64 | `rvt-rs-<version>-x86_64-pc-windows-msvc.zip` |

Each archive holds the binaries plus `README.md`, `LICENSE` and `NOTICE`.
Download `SHA256SUMS` from the same release and check the archive before
unpacking:

```bash
# Linux / macOS
shasum -a 256 -c SHA256SUMS --ignore-missing
tar -xzf rvt-rs-<version>-<target>.tar.gz
./rvt-rs-<version>-<target>/rvt-info --version
```

```powershell
# Windows (PowerShell): compare with the line for your archive in SHA256SUMS
Get-FileHash .\rvt-rs-<version>-x86_64-pc-windows-msvc.zip -Algorithm SHA256
Expand-Archive .\rvt-rs-<version>-x86_64-pc-windows-msvc.zip -DestinationPath .
.\rvt-rs-<version>-x86_64-pc-windows-msvc\rvt-info.exe --version
```

The binaries are not code-signed. On macOS, a browser download is
quarantined by Gatekeeper; clear it for the unpacked folder with
`xattr -dr com.apple.quarantine rvt-rs-<version>-<target>`. On Windows,
SmartScreen may ask for confirmation the first time.

To run the tools from anywhere, move the folder somewhere permanent and add
it to your `PATH`.

Between releases, any commit can be built the same way from the **Release
binaries** workflow (Actions tab → Release binaries → Run workflow); the
archives are attached to that run as artifacts.

## Docker

Every tagged release from the next one on also publishes a container image
holding the same static Linux binaries, for `linux/amd64` and `linux/arm64`:
`ghcr.io/drunkonjava/rvt-rs:<version>` and, for a final release,
`ghcr.io/drunkonjava/rvt-rs:latest`. It is a distroless image of about the
binaries' size, with no shell and no package manager. The working directory
is `/data`, so mount the folder holding the model there:

```bash
docker run --rm -v "$PWD:/data" --user "$(id -u):$(id -g)" \
  ghcr.io/drunkonjava/rvt-rs rvt-info model.rvt
docker run --rm -v "$PWD:/data" --user "$(id -u):$(id -g)" \
  ghcr.io/drunkonjava/rvt-rs rvt-ifc model.rvt -o model.ifc
```

`--user` makes files the tools write owned by you rather than by the image's
default non-root user. Without a command the image prints `rvt-info --help`.

To build the image yourself, stage the release's Linux archives and build
from `docker/`:

```bash
gh release download <tag> --repo DrunkOnJava/rvt-rs -p '*linux-musl*' -p SHA256SUMS -D dist
docker/stage.sh dist
docker buildx build --platform linux/amd64,linux/arm64 -f docker/Dockerfile docker/context
```

## Build From Source

Use the source path for Rust CLIs (and for testing unreleased commits).

```bash
git clone https://github.com/DrunkOnJava/rvt-rs
cd rvt-rs
cargo build --release
./target/release/rvt-inspect --version
```

Run a local source smoke test:

```bash
./target/release/rvt-inspect path/to/model.rvt
./target/release/rvt-ifc path/to/model.rvt -o model.ifc --mode strict \
  --diagnostics model.diagnostics.json
```

## Build Python From Source

Use `maturin` when testing Python bindings from a source checkout:

```bash
python -m venv .venv
. .venv/bin/activate
python -m pip install --upgrade pip maturin
maturin develop --manifest-path rvt-py/Cargo.toml
python -c "import rvt; print(rvt.__version__)"
```

Build a wheel instead of installing into the active environment:

```bash
maturin build --release --manifest-path rvt-py/Cargo.toml --out dist
python -m pip install dist/rvt-*.whl
```

## Build The Viewer Locally

The viewer needs a WASM package and Node dependencies:

```bash
wasm-pack build --target web --out-dir viewer/pkg -- --features wasm --no-default-features
cd viewer
npm install
npm run typecheck
npm run build
```

`npm run build` writes the static site to `viewer/dist`. Use
`npm run dev` for local development.

## Post-Publish Verification

Release managers should verify every published artifact from a clean shell. The
full release gate is in [release-checklist.md](release-checklist.md); the short
post-publish smoke is:

```bash
# Only after crates.io publish succeeds:
cargo install rvt --version X.Y.Z --locked
rvt-inspect --version

python -m venv /tmp/rvt-release-smoke
. /tmp/rvt-release-smoke/bin/activate
python -m pip install --upgrade pip
python -m pip install "rvt==X.Y.Z"
python -c "import rvt; print(rvt.__version__)"
```

Then open <https://drunkonjava.github.io/rvt-rs/> and confirm the viewer loads.
If a sample file is available, run `rvt-inspect` and the viewer diagnostics
download against the same file and compare the failure mode.

Do **not** mark docs.rs as PASS unless `https://docs.rs/rvt/X.Y.Z/rvt/`
returns HTTP 200.
