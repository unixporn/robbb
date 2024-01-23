FROM debian:bookworm-slim

RUN apt-get -y update && apt-get -y install ca-certificates wget \
  && rm -rf /var/lib/apt/lists/*
# gdb heaptrack \

RUN wget https://github.com/koute/bytehound/releases/download/0.11.0/bytehound-x86_64-unknown-linux-gnu.tgz \
  && tar -xvzf bytehound-x86_64-unknown-linux-gnu.tgz

COPY ./robbb /usr/local/bin/robbb
RUN chmod +x /usr/local/bin/robbb

ENV LD_PRELOAD=./libbytehound.so
CMD ["robbb"]
