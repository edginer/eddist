FROM rust:1.79

RUN mkdir /eddist-build
WORKDIR /eddist-build
COPY ./ ./

RUN cargo b --release

FROM debian:bookworm-slim

ENV RUST_ENV=prod
ENV RUST_LOG=info

COPY --from=0 /eddist-build/target/release/eddist .
EXPOSE 8080

ENTRYPOINT [ "./eddist" ]
