FROM ekidd/rust-musl-builder AS builder

# cache dependencies
ADD --chown=rust:rust Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" >src/main.rs && cargo build --release && rm -rf src

ADD --chown=rust:rust sqlx-data.json .
ADD --chown=rust:rust src src
# cargo doesn't rebuild without this
RUN printf "\n// nothing" >>src/main.rs

RUN cargo build --release

FROM alpine

WORKDIR /usr/local/share/app

COPY --from=builder /home/rust/src/target/x86_64-unknown-linux-musl/release/trup-rs .

CMD ./trup-rs
