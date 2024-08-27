CREATE TABLE
    messsages (
        id SERIAL PRIMARY KEY,
        username varchar(128) NOT NULL,
        message TEXT NOT NULL,
        timestamp BIGINT NOT NULL DEFAULT EXTRACT(
            'epoch'
            FROM
                CURRENY_TIMESTAMP
        )
    )