FROM debian:bookworm-slim

COPY ./target/release/eddist-persistence .

ENV RUST_ENV=prod
ENV RUST_LOG=info

EXPOSE 9874

ENTRYPOINT [ "./eddist-persistence" ]
