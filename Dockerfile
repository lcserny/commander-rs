# quick and dirty docker container

FROM rust:1.76.0

WORKDIR /commander

COPY target/release/commander .

COPY config ./config

EXPOSE 8090

VOLUME ["/data"]

CMD ["./commander"]