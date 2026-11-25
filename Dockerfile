FROM ubuntu:24.04 AS build
RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential ca-certificates cmake curl perl pkg-config \
    && rm -rf /var/lib/apt/lists/*
ENV CARGO_HOME=/opt/cargo RUSTUP_HOME=/opt/rustup
ENV PATH="/opt/cargo/bin:${PATH}"
WORKDIR /build
COPY rust-toolchain.toml Cargo.toml Cargo.lock ./
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup.sh \
    && sh /tmp/rustup.sh -y --no-modify-path --default-toolchain none \
    && rm /tmp/rustup.sh \
    && rustup show
COPY .cargo .cargo
COPY src src
COPY assets/beyond-gate-logo-braille.txt assets/beyond-gate-logo-braille.txt
RUN cargo build --release --workspace --locked

FROM ubuntu:24.04 AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 10001 --create-home beyond-gate
COPY --from=build /build/target/release/beyond-gate /build/target/release/beyond-gate-mcp /usr/local/bin/
USER 10001:10001
WORKDIR /home/beyond-gate
ENTRYPOINT ["beyond-gate"]
