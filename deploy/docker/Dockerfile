# This code is based on rust-playground container:
# https://github.com/integer32llc/rust-playground/blob/master/compiler/base/Dockerfile

FROM ubuntu:20.04 as toolchain

ENV DEBIAN_FRONTEND="noninteractive"

# `build-essential` and `file` are needed for backtrace-sys
# `cmake`, `git`, `python` are needed for wasm tools
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    curl \
    file \
    gcc \
    git \
    libssl-dev \
    pkg-config \
    python \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m playground -d /playground
RUN usermod -p '!!' root # Disable all passwords for root
USER playground
ENV USER=playground
ENV PATH=/playground/.cargo/bin:$PATH
WORKDIR /playground

# Ensure that we are using the latest stable version of rustup and the
# latest version of the current channel. A new manifest will trigger
# these lines to run again, forcing a new download of rustup and
# installation of Rust.
ADD --chown=playground https://static.rust-lang.org/rustup/release-stable.toml /playground/tools/rustup-manifest.toml
ADD --chown=playground https://static.rust-lang.org/dist/channel-rust-stable-date.txt /playground/tools/rust-channel-version
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal --default-toolchain "stable"

# Install wasm target and tools
RUN rustup target add wasm32-unknown-unknown
RUN cd /tmp && \
    git clone https://github.com/WebAssembly/wabt && \
    cd /tmp/wabt/ && \
    cmake -DBUILD_TESTS=OFF -DCMAKE_BUILD_TYPE=Release && \
    make wasm-strip && \
    cp ./wasm-strip $HOME/.cargo/bin && \
    rm -rf /tmp/wabt/

ADD --chown=playground rustc-wasm /playground/.cargo/bin/
ADD --chown=playground entrypoint.sh /playground/tools/

ENTRYPOINT ["/playground/tools/entrypoint.sh"]
