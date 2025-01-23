FROM rust:alpine AS chef

RUN apk update && apk add musl-dev && cargo install cargo-chef

WORKDIR /src/app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /src/app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release

FROM scratch

COPY --from=builder /src/app/target/release/get-tl-mirror-status .

USER 1000:1000
ENTRYPOINT ["./get-tl-mirror-status"]
