FROM rust:1.79

RUN mkdir /eddist-build
WORKDIR /eddist-build
COPY ./ ./

RUN cargo b --release

FROM debian:bookworm-slim

COPY --from=0 /eddist-build/target/release/eddist .
EXPOSE 8080

ENTRYPOINT [ "./eddist" ]
