FROM postgres:17

WORKDIR /

RUN apt-get update && apt-get install -y git make build-essential

RUN git clone https://github.com/wolfcw/libfaketime.git

WORKDIR /libfaketime/src
RUN make install

# ENV LD_PRELOAD=/usr/local/lib/faketime/libfaketime.so.1
# ENV FAKETIME_NO_CACHE=1
# ENV DONT_FAKE_MONOTONIC=1
