FROM rust:1.70

RUN rustup target add wasm32-unknown-unknown
RUN cargo install --locked trunk

WORKDIR /app
COPY . .

RUN trunk build --release kgc_server/frontend/index.html
RUN cp -r kgc_server/frontend/dist ./dist

WORKDIR /app
RUN cargo build -r --bin kgc_server
RUN cp ./target/release/kgc_server ./kgc
RUN cargo clean

CMD ["./kgc"]
