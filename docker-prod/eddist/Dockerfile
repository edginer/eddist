FROM debian:bookworm-slim

COPY ./target/release/eddist .
COPY ./eddist-server/client/dist ./dist

ENV RUST_ENV=prod
# for debugging
ENV RUST_LOG=debug
ENV RUST_BACKTRACE=verbose
EXPOSE 8080

ENTRYPOINT [ "./eddist" ]
