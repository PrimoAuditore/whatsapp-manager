FROM rust:slim as build

RUN apt-get update && \
    apt-get install -y \
        git \
        openssh-server \
        openssh-client

ENV META_TOKEN=""
ENV REDIS_URL=""
ARG SSH_KEY

WORKDIR /app
COPY . .
RUN --mount=type=ssh cargo build --release


FROM debian:11-slim
WORKDIR /app
COPY --from=build /app/target/release/whatsapp-manager ./whatsapp-manager
CMD ["./whatsapp-manager"]