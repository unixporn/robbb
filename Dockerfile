FROM debian:bullseye-slim

COPY ./robbb /usr/local/bin/robbb

CMD ["robbb"]
