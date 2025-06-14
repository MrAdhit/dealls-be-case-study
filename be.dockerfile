FROM rust:1.87 AS builder
WORKDIR /
COPY . .
RUN cargo install --path .

FROM debian:bookworm
RUN apt-get update && apt-get install -y git make build-essential && rm -rf /var/lib/apt/lists/*

RUN git clone https://github.com/wolfcw/libfaketime.git

WORKDIR /libfaketime/src
RUN make install

COPY --from=builder /usr/local/cargo/bin/dealls-be-case-study /usr/local/bin/app

ENV LD_PRELOAD=/usr/local/lib/faketime/libfaketime.so.1
ENV FAKETIME_NO_CACHE=1

CMD ["app"]
