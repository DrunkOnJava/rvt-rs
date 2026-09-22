#!/usr/bin/env bash
# Stage the Docker build context from a release's static Linux CLI archives.
#
#   docker/stage.sh <dir>
#
# <dir> holds rvt-rs-<version>-x86_64-unknown-linux-musl.tar.gz,
# rvt-rs-<version>-aarch64-unknown-linux-musl.tar.gz and SHA256SUMS: the
# release-binaries workflow's artifacts, or a release's assets
# (`gh release download v0.2.0 -p '*linux-musl*' -p SHA256SUMS -D dist`).
# The archives are checked against SHA256SUMS, then unpacked into
# docker/context/linux-amd64/, docker/context/linux-arm64/ and, once,
# docker/context/doc/ (README.md, LICENSE, NOTICE).
set -euo pipefail

dist="${1:?usage: docker/stage.sh <dir with the linux-musl archives and SHA256SUMS>}"
dist="$(cd "${dist}" && pwd)"
here="$(cd "$(dirname "$0")" && pwd)"
ctx="${here}/context"

(cd "${dist}" && shasum -a 256 -c --ignore-missing SHA256SUMS)

rm -rf "${ctx}"
mkdir -p "${ctx}/doc"
scratch="$(mktemp -d)"
trap 'rm -rf "${scratch}"' EXIT

for pair in "amd64:x86_64-unknown-linux-musl" "arm64:aarch64-unknown-linux-musl"; do
  arch="${pair%%:*}"
  target="${pair#*:}"
  archives=("${dist}"/rvt-rs-*-"${target}".tar.gz)
  if [[ ${#archives[@]} -ne 1 || ! -f "${archives[0]}" ]]; then
    echo "error: expected exactly one rvt-rs-*-${target}.tar.gz in ${dist}" >&2
    exit 1
  fi
  if ! grep -q " $(basename "${archives[0]}")\$" "${dist}/SHA256SUMS"; then
    echo "error: $(basename "${archives[0]}") is not listed in SHA256SUMS" >&2
    exit 1
  fi
  mkdir -p "${scratch}/${arch}" "${ctx}/linux-${arch}"
  tar -xzf "${archives[0]}" -C "${scratch}/${arch}" --strip-components=1
  for doc in README.md LICENSE NOTICE; do
    mv "${scratch}/${arch}/${doc}" "${ctx}/doc/${doc}"
  done
  mv "${scratch}/${arch}"/* "${ctx}/linux-${arch}/"
done

echo "staged $(ls "${ctx}/linux-amd64" | wc -l | tr -d ' ') binaries per architecture in ${ctx}"
