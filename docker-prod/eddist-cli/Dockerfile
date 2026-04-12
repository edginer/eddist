FROM debian:bookworm-slim

RUN apt update && apt install -y --no-install-recommends ca-certificates libc6
COPY ./target/release/eddist-cli .

ENV RUST_LOG=info

ENTRYPOINT [ "./eddist-cli" ]
