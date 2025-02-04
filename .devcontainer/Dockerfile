# select base image
FROM ubuntu:latest

# Install dependencies
RUN dpkg --add-architecture i386 && \
    apt update && apt install -y \
    sudo man-db apt-utils dialog \
    git curl build-essential \
    protobuf-compiler && \
    rm -rf /var/lib/apt/lists/* && \
    apt update && \
    dpkg -l | grep ^ii | cut -d' ' -f3 | xargs apt-get install -y --reinstall && \
    rm -rf /var/lib/apt/lists/* && \
    apt update && \
    dpkg -l | grep ^ii | cut -d' ' -f3 | xargs apt-get install -y --reinstall && \
    rm -rf /var/lib/apt/lists/*

# Remove default (ubuntu) user
RUN if id "ubuntu" &>/dev/null; then \
        userdel -r ubuntu; \
    fi

# Add dev user
RUN useradd -m -G sudo dev && \
    echo "dev ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

# Setup workspace
RUN mkdir /workspace && chown -R dev:dev /workspace

# Switch to dev user
USER dev

# Install rustup and Rust nightly version
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain nightly -y
