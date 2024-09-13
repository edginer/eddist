FROM debian:bookworm-slim

COPY ./target/release/eddist-admin .
COPY ./eddist-admin/client/build/client ./dist

ENV RUST_ENV=prod
ENV RUST_LOG=info
EXPOSE 8080

ENTRYPOINT [ "./eddist-admin" ]
