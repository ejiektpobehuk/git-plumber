FROM scratch
LABEL org.opencontainers.image.source=https://github.com/ejiektpobehuk/git-plumber
LABEL org.opencontainers.image.description="Bare image of git-plumber"
LABEL org.opencontainers.image.licenses=MIT

ARG TARGETARCH
COPY ${TARGETARCH}/git-plumber /git-plumber

WORKDIR /workspace

ENTRYPOINT ["/git-plumber"]
