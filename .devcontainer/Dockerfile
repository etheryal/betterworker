FROM mcr.microsoft.com/devcontainers/rust:1-1-bookworm

RUN apt-get update \
     && curl -s https://packagecloud.io/install/repositories/github/git-lfs/script.deb.sh | sudo bash \
     && apt-get install -y git-lfs libssl-dev pkg-config

USER vscode
RUN curl -fsSL https://bun.sh/install | bash