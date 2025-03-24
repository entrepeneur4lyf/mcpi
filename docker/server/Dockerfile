FROM rust:1 as build-env
WORKDIR /app
COPY . /app
RUN cargo build --release --bin mcpi-server
FROM gcr.io/distroless/cc-debian12
COPY --from=build-env /app/target/release/mcpi-server /
COPY --from=build-env /app/data /data
EXPOSE 3001
CMD ["./mcpi-server"]