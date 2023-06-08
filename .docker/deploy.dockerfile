FROM rust:1.70

WORKDIR /app
COPY . .

RUN cargo build -r
RUN cp ./target/release/karlsruhe-garbage-calendar ./
RUN cargo clean

CMD ["./karlsruhe-garbage-calendar"]
