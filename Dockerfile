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
#RUN echo
#RUN echo $SSH_KEY > private_key_encoded
#RUN base64 -d private_key_encoded
#RUN base64 -d private_key_encoded > private_key && chmod 600 private_key
#RUN cat private_key
#RUN eval `ssh-agent -s` && ssh-add ./private_key && rm private_key private_key_encoded

RUN --mount=type=ssh cargo build --release


FROM debian:11-slim
WORKDIR /app
COPY --from=build /app/target/release/whatsapp-manager ./whatsapp-manager
CMD ["./whatsapp-manager"]