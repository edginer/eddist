FROM debian:bookworm-slim

COPY ./target/release/eddist .
COPY ./eddist-server/client/dist ./dist

ENV RUST_ENV=prod
ENV RUST_LOG=info
EXPOSE 8080

ENTRYPOINT [ "./eddist" ]
