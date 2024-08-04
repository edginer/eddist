FROM debian:bookworm-slim

COPY ./target/release/eddist .

ENV RUST_ENV=prod
ENV RUST_LOG=info
EXPOSE 8080

ENTRYPOINT [ "./eddist" ]
