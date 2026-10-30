FROM mcr.microsoft.com/devcontainers/base:ubuntu-24.04

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential ca-certificates cmake curl perl pkg-config \
    && apt-get clean

USER vscode
ENV CARGO_HOME=/home/vscode/.cargo \
    RUSTUP_HOME=/home/vscode/.rustup
ENV PATH="/home/vscode/.cargo/bin:${PATH}"

# Select the checkout's toolchain only after the workspace is mounted.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/beyond-gate-rustup.sh \
    && sh /tmp/beyond-gate-rustup.sh -y --no-modify-path --default-toolchain none \
    && rm /tmp/beyond-gate-rustup.sh
