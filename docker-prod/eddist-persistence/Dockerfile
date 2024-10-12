FROM debian:bookworm-slim

RUN apt update && apt install -y --no-install-recommends ca-certificates libc6

COPY ./target/release/eddist-persistence .

ENV RUST_ENV=prod
ENV RUST_LOG=info

EXPOSE 9874

ENTRYPOINT [ "./eddist-persistence" ]
