FROM rust:1.70-bookworm AS base

RUN cargo install cargo-watch
RUN cargo install --locked trunk

RUN rustup target add wasm32-unknown-unknown

FROM base AS trunk_builder

WORKDIR /opt/build
COPY . .
RUN trunk build --release kgc_server/frontend/index.html

FROM base AS cargo_builder

WORKDIR /opt/build
COPY . .
RUN cargo build -r --bin kgc_server

FROM debian:bookworm-slim AS build

WORKDIR /app
COPY --from=trunk_builder /opt/build/kgc_server/frontend/dist ./dist
COPY --from=cargo_builder /opt/build/target/release/kgc_server ./kgc_server

CMD ["/app/kgc_server"]