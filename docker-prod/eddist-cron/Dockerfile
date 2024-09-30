FROM debian:bookworm-slim

RUN apt update && apt install -y --no-install-recommends ca-certificates
COPY ./target/release/eddist-cron .

ENV RUST_ENV=prod
ENV RUST_LOG=info
EXPOSE 8080

ENTRYPOINT [ "./eddist-cron" ]