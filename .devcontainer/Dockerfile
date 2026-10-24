FROM mcr.microsoft.com/devcontainers/base:ubuntu-24.04@sha256:d94c97dd9cacf183d0a6fd12a8e87b526e9e928307674ae9c94139139c0c6eae

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential ca-certificates curl git-lfs libssl-dev pkg-config \
    python3-dev python3-venv shellcheck \
    && rm -rf /var/lib/apt/lists/*

ARG DEV_USER=vscode
ENV PATH="/home/${DEV_USER}/.local/bin:${PATH}"
COPY rvt-env.sh /etc/profile.d/rvt-env.sh
