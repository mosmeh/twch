FROM rust:1.52.1 as builder
WORKDIR /repo
COPY . .
RUN cargo build -p twch-server --release

FROM rust:1.52.1-slim
RUN useradd -m -u 1000 docker
USER 1000
COPY --from=builder /repo/target/release/twch-server .
EXPOSE 8080
ENTRYPOINT ["./twch-server"]
