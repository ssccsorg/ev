FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    python3 \
    yosys \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Verify toolchain
RUN yosys --version && rustc --version

WORKDIR /workspace
COPY . .

# ssccs-core is a path dependency used only in CI for channel demo.
# It is cloned in the CI workflow, not here.
# To build locally with ssccs:  ln -s /path/to/ssccs ../ssccs
RUN cargo build --release

# Smoke test: synthesis with mock
RUN EV_SYNTH_BACKEND=mock ./target/release/ev check \
    --target tests/fixtures/all_pass.xif.yaml --synth

# Real synthesis (Yosys)
RUN ./target/release/ev check \
    --target tests/fixtures/all_pass.xif.yaml --synth

ENTRYPOINT ["./target/release/ev"]
