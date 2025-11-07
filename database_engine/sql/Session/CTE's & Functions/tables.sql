-- RENAME: Moving away from the hardcoded US2000 name

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
-- Update Index names for generalization
CREATE INDEX market_data_time_idx ON public.market_data USING btree ("time" DESC);

DROP INDEX idx_asset_id;

CREATE INDEX idx_asset_id ON public.market_data USING btree (asset_id);

---------------------------------
-- Session Context Table
---------------------------------

DROP TABLE IF EXISTS new_session_context CASCADE;
CREATE TABLE IF NOT EXISTS new_session_context (
    -- Primary Context (CS is the focus session for prediction)
    cs_ps1_fk BIGINT NOT NULL,          -- PK for PS1 -> CS link
    trading_date DATE NOT NULL,
    asset_id TEXT NOT NULL,
    -- Current Session (CS) Details
    cs_name TEXT NOT NULL,
    cs_bias_3_state TEXT,
    cs_bias_7_state TEXT,
    -- Preceding Session 1 (PS1) Details
    ps1_name TEXT NOT NULL,
    ps1_bias_3_state TEXT,
    ps1_bias_7_state TEXT,
    -- Preceding Session 2 (PS2) Details
    ps2_name TEXT,
    ps2_bias_3_state TEXT,
    ps2_bias_7_state TEXT,
    -- Foreign Key for the PS2 -> PS1 link (used by TCS)
    ps2_ps1_fk BIGINT,                  -- FK for PS2 -> PS1 link
    -- Foreign Key for the entire PS2 -> PS1 -> CS link (used by 2nd-Order metrics)
    cs_ps2_fk BIGINT,                   -- FK for PS2 -> PS1 -> CS link
    -- Constraints
    PRIMARY KEY(cs_ps1_fk)
);
-- Create the crucial foreign key columns as sequences
CREATE SEQUENCE IF NOT EXISTS cs_ps1_fk_seq START 1;
CREATE SEQUENCE IF NOT EXISTS ps2_ps1_fk_seq START 1;
CREATE SEQUENCE IF NOT EXISTS cs_ps2_fk_seq START 1;

TRUNCATE asset_session_context;


--------------------------------
-- Session Views Table
--------------------------------
DROP TABLE IF EXISTS session_views;
CREATE TABLE session_views (
    trading_date date NOT NULL,
    asset_id text NOT NULL,
    session_name text NOT NULL,
    start_ts timestamp with time zone NOT NULL,
    end_ts timestamp with time zone NOT NULL,
    session_type text,
    consolidation_subtype text,
    open double precision,
    high double precision,
    low double precision,
    close double precision,
    volume bigint,
    bars bigint,
    PRIMARY KEY(trading_date, asset_id, session_name),
    -- Foreign Key Constraint
    FOREIGN KEY (asset_id) REFERENCES assets(id)
);
TRUNCATE session_views;


