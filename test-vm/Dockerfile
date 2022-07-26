FROM rust:latest

WORKDIR /usr/src/near-contract-tools
COPY . .

WORKDIR /usr/src/near-contract-tools/test-vm

RUN cargo install --path .

CMD [ "test-vm" ]
