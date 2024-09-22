FROM rust

WORKDIR /usr/src/messages_microservices
COPY . .

RUN cargo install --path .
RUN echo "test"

CMD [ "messages_microservices/target/debug/build" ]