FROM debian:bullseye-slim

RUN apt-get -y update && apt-get -y install ca-certificates && rm -rf /var/lib/apt/lists/*

COPY ./robbb /usr/local/bin/robbb
RUN chmod +x /usr/local/bin/robbb

CMD ["robbb"]
