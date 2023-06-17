FROM rust:1.70

RUN cargo install cargo-watch
RUN rustup target add wasm32-unknown-unknown
RUN cargo install --locked trunk
