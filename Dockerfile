# syntax=docker/dockerfile:1
# Build context: a directory with this Dockerfile plus amd64/dt and arm64/dt
# (statically linked musl binaries; see .github/workflows/release.yml).
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends git openssh-client ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 1000 dt
ARG TARGETARCH
COPY --chmod=0755 ${TARGETARCH}/dt /usr/local/bin/dt
USER dt
WORKDIR /workspace
ENTRYPOINT ["dt"]
