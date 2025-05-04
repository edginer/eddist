FROM debian:bookworm-slim

RUN apt update && apt install -y --no-install-recommends ca-certificates libc6

COPY ./target/release/eddist .
COPY ./eddist-server/client/dist ./dist
COPY ./eddist-server/resources ./resources

ENV RUST_ENV=prod
ENV RUST_LOG=info
EXPOSE 8080

ENTRYPOINT [ "./eddist" ]
