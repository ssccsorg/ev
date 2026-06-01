FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

# Rust + Yosys + RISC-V cross-compiler + Spike + pk
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    device-tree-compiler \
    git \
    python3 \
    yosys \
    gcc-riscv64-linux-gnu \
    binutils-riscv64-linux-gnu \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustc --version && yosys --version

# RISC-V target for Rust cross-compilation (if needed)
RUN rustup target add riscv64imac-unknown-none-elf

# Cross-compiler alias (riscv-pk and our scripts expect riscv64-unknown-elf-gcc)
RUN ln -sf /usr/bin/riscv64-linux-gnu-gcc /usr/local/bin/riscv64-unknown-elf-gcc \
    && ln -sf /usr/bin/riscv64-linux-gnu-g++ /usr/local/bin/riscv64-unknown-elf-g++ \
    && ln -sf /usr/bin/riscv64-linux-gnu-ar /usr/local/bin/riscv64-unknown-elf-ar \
    && ln -sf /usr/bin/riscv64-linux-gnu-as /usr/local/bin/riscv64-unknown-elf-as \
    && ln -sf /usr/bin/riscv64-linux-gnu-ld /usr/local/bin/riscv64-unknown-elf-ld \
    && ln -sf /usr/bin/riscv64-linux-gnu-objdump /usr/local/bin/riscv64-unknown-elf-objdump \
    && ln -sf /usr/bin/riscv64-linux-gnu-objcopy /usr/local/bin/riscv64-unknown-elf-objcopy

# Verify cross-compiler
RUN echo 'int main(){}' | riscv64-unknown-elf-gcc -x c - -o /tmp/test && rm /tmp/test

# Spike from source
RUN git clone https://github.com/riscv-software-src/riscv-isa-sim.git /tmp/spike \
    && cd /tmp/spike && mkdir build && cd build \
    && ../configure --prefix=/usr/local && make -j$(nproc) && make install \
    && rm -rf /tmp/spike

# riscv-pk from source, cross-compiled for RISC-V
RUN git clone https://github.com/riscv-software-src/riscv-pk.git /tmp/pk \
    && cd /tmp/pk && mkdir build && cd build \
    && CC=riscv64-unknown-elf-gcc ../configure --prefix=/usr/local --host=riscv64-unknown-elf \
    && make -j$(nproc) && make install \
    && rm -rf /tmp/pk

# Verify pk binary
RUN test -f /usr/local/riscv64-unknown-elf/bin/pk

WORKDIR /workspace

# Build-time: compile ev inside the image
COPY . .
RUN cargo build --release

# Smoke tests
RUN EV_SYNTH_BACKEND=mock ./target/release/ev verify \
    --target tests/fixtures/all_pass.xif.yaml
RUN ./target/release/ev synth \
    --target tests/fixtures/all_pass.xif.yaml
RUN EV_SIM_BACKEND=spike EV_PK_PATH=/usr/local/riscv64-unknown-elf/bin/pk \
    ./target/release/ev simulate \
    --target tests/fixtures/all_pass.xif.yaml

ENTRYPOINT ["./target/release/ev"]
