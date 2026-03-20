FROM rust:latest

WORKDIR /parser

RUN apt-get update && \
    apt-get install -y protobuf-compiler && \
    rm -rf /var/lib/apt/lists/*

# Use UID 1000 to match devcontainer (avoids permission issues with mounted volumes)
RUN adduser -u 1000 --disabled-password --gecos "" lifted && chown -R lifted /parser

# Switch to lifted user for cargo operations
USER lifted

RUN cargo install cargo-watch
RUN mkdir -p /parser/src/compressed-replays
RUN mkdir -p /parser/src/replays

COPY --chown=lifted:lifted . /parser/

RUN rustc --version
RUN protoc --version

CMD ["cargo", "watch", "-i", "src/compressed-replays/**", "-i", "src/replays/**", "-x", "run"]
