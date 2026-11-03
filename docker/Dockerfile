# rvt-rs CLIs on a distroless static base.
#
# The binaries are the release's own static musl builds, smoke-tested by
# .github/workflows/release-binaries.yml, so this image compiles nothing and
# builds for arm64 without emulation. Stage the context first:
#
#   docker/stage.sh dist
#   docker buildx build --platform linux/amd64,linux/arm64 -f docker/Dockerfile docker/context
#
# Run with the model's folder mounted at /data (the working directory):
#
#   docker run --rm -v "$PWD:/data" --user "$(id -u):$(id -g)" \
#     ghcr.io/drunkonjava/rvt-rs rvt-ifc model.rvt -o model.ifc
FROM gcr.io/distroless/static-debian13:nonroot@sha256:25ac8f0c3b2334a9eb05452266bd4697864a8c8ef0ef71e5e6480d7f92868329

ARG TARGETARCH
ARG VERSION=unknown

LABEL org.opencontainers.image.title="rvt-rs" \
      org.opencontainers.image.description="Clean-room CLIs for inspecting Autodesk Revit files (.rvt, .rfa, .rte, .rft) without Revit" \
      org.opencontainers.image.source="https://github.com/DrunkOnJava/rvt-rs" \
      org.opencontainers.image.documentation="https://github.com/DrunkOnJava/rvt-rs/blob/main/docs/install.md#docker" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.version="${VERSION}"

COPY doc/ /usr/share/doc/rvt-rs/
COPY linux-${TARGETARCH}/ /usr/local/bin/

WORKDIR /data
CMD ["rvt-info", "--help"]
