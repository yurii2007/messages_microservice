-- Your SQL goes here
CREATE TABLE
    messages (
        id SERIAL PRIMARY KEY,
        username varchar(128) NOT NULL,
        message TEXT NOT NULL,
        timestamp BIGINT NOT NULL DEFAULT EXTRACT(
            'epoch'
            FROM
                CURRENT_TIMESTAMP
        )
    )