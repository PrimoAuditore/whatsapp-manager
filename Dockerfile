FROM rust:slim as build

ENV META_TOKEN=""
ENV REDIS_URL=""
ENV MEDIA_BUCKET=""

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:11-slim
RUN apt update && apt install -y ca-certificates curl
WORKDIR /app
COPY --from=build /app/target/release/vin-webhook ./vin-webhook
COPY --from=build /app/download-image.sh ./download-image.sh
CMD ["./vin-webhook"]