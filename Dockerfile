FROM rust:latest

RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    git \
    && rm -rf /var/lib/apt/lists/*

# Set Rust edition 2024
ENV RUSTFLAGS="--edition 2024"

# Install nightly for 2024 edition support
RUN rustup default nightly && \
    rustup component add rustfmt clippy rust-src

WORKDIR /app
COPY . .

RUN cargo build --workspace

CMD ["cargo", "run"]