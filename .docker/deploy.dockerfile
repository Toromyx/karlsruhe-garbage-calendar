FROM rust:1.70

WORKDIR /app
COPY . .

RUN cargo build -r --bin kgc_server
RUN cp ./target/release/kgc_server ./kgc
RUN cargo clean

CMD ["./kgc"]
