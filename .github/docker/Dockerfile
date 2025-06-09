FROM scratch

ARG TARGETARCH
COPY ${TARGETARCH}/git-plumber /git-plumber

WORKDIR /workspace

ENTRYPOINT ["/git-plumber"]
