----------------------------
--- Market Data view
-------------------------

DROP TABLE market_data;

CREATE TABLE market_data (
    time timestamp with time zone NOT NULL,
    asset_id text NOT NULL,
    "open" double precision NOT NULL,
    high double precision NOT NULL,
    low double precision NOT NULL,
    close double precision NOT NULL,
    volume double precision NOT NULL,
    PRIMARY KEY(time, asset_id),
    -- <<< ADD THE FOREIGN KEY CONSTRAINT >>>
    FOREIGN KEY (asset_id) REFERENCES assets(id)
);

TRUNCATE TABLE market_data;

-- Update Index names for generalization

SELECT create_hypertable('market_data', 'time', if_not_exists => TRUE);
CREATE INDEX market_data_time_idx ON public.market_data USING btree ("time" DESC);

DROP INDEX idx_asset_id;

CREATE INDEX idx_asset_id ON public.market_data USING btree (asset_id);
