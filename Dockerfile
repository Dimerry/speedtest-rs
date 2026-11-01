FROM scratch
COPY target/x86_64-unknown-linux-musl/release/speedtest-rs /speedtest-rs
ENV PORT=5111
EXPOSE 5111
ENTRYPOINT ["/speedtest-rs"]
