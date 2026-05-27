FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

# Rust + Yosys + tools
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    python3 \
    yosys \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustc --version && yosys --version

WORKDIR /workspace

# Build-time: compile ev inside the image so the image is self-contained
COPY . .
RUN cargo build --release

# Smoke tests (synthesis with mock and real Yosys)
RUN EV_SYNTH_BACKEND=mock ./target/release/ev check \
    --target tests/fixtures/all_pass.xif.yaml --synth
RUN ./target/release/ev check \
    --target tests/fixtures/all_pass.xif.yaml --synth

ENTRYPOINT ["./target/release/ev"]
