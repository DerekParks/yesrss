FROM docker.io/library/rust:1.98-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src src
COPY migrations migrations
COPY static static
RUN cargo build --release --locked && mkdir /data

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /src/target/release/yesrss /usr/local/bin/yesrss
COPY --from=build --chown=nonroot:nonroot /data /data
ENV YESRSS_LISTEN=0.0.0.0:8080 \
    YESRSS_DB=/data/yesrss.db
VOLUME /data
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/usr/local/bin/yesrss"]
CMD ["serve"]
