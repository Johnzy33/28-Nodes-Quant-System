-------------------
--- 12 to 15
-- STRUCTURAL BATTLE INDEX (SBI) MODEL

-- asset_session_context (The primary join table for pattern context)
CREATE TABLE asset_session_context (
    trading_date date NOT NULL,
    asset_id text NOT NULL,
    cs_ps1_fk BIGINT NOT NULL PRIMARY KEY, -- Unique ID for the CS/PS1 pattern
    ps1_name text NOT NULL,
    cs_name text NOT NULL,
    ps1_bias_7_state text NOT NULL,
    ...
);

-- M12: Structural Battle Index (SBI)
CREATE TABLE asset_structural_battle_index (
    asset_id TEXT NOT NULL,
    ps1_name TEXT NOT NULL,
    ps1_bias_7 TEXT NOT NULL,
    cs_name TEXT NOT NULL, 
    prior_level TEXT NOT NULL,
    total_breaks BIGINT NOT NULL,
    avg_battle_duration_s DOUBLE PRECISION NOT NULL, -- M12 Component 1
    avg_battle_range_pct DOUBLE PRECISION NOT NULL,  -- M12 Component 2
    battle_label TEXT NOT NULL,
    PRIMARY KEY (asset_id, ps1_name, ps1_bias_7, cs_name, prior_level)
);

-- M13: Daily ATR (Normalization)
CREATE TABLE asset_daily_atr_14d (
    asset_id TEXT NOT NULL,
    trading_date DATE NOT NULL,
    true_range DOUBLE PRECISION NOT NULL,
    daily_atr DOUBLE PRECISION NOT NULL, -- The normalized value
    PRIMARY KEY (asset_id, trading_date)
);

-- M14: Open Interest Takedown (OIT)
CREATE TABLE asset_open_interest_takedown (
    asset_id TEXT NOT NULL,
    cs_ps1_fk BIGINT NOT NULL,
    ps1_bias_7 TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    trend_direction TEXT NOT NULL,
    total_patterns BIGINT NOT NULL,
    oit_takedown_count BIGINT NOT NULL,
    oit_score DOUBLE PRECISION NOT NULL, -- M14 Result
    avg_pullback_pct DOUBLE PRECISION NOT NULL,
    oit_label TEXT NOT NULL,
    PRIMARY KEY (cs_ps1_fk, trend_direction)
);

-- Aggregated Scores
CREATE TABLE asset_pdrs_score (
    asset_id TEXT NOT NULL, cs_ps1_fk BIGINT NOT NULL, prior_level TEXT NOT NULL, 
    p_takedown DOUBLE PRECISION NOT NULL, sbi_duration_factor DOUBLE PRECISION NOT NULL,
    pdrs_score DOUBLE PRECISION NOT NULL, -- P.D.R.S. (Idea 3)
    PRIMARY KEY (cs_ps1_fk, prior_level)
);

CREATE TABLE asset_cbes_score (
    asset_id TEXT NOT NULL, cs_ps1_fk BIGINT NOT NULL, trade_direction TEXT NOT NULL,
    cvi_score DOUBLE PRECISION NOT NULL, p_follow_through DOUBLE PRECISION NOT NULL,
    cbes_score DOUBLE PRECISION NOT NULL, -- C.B.E.S. (Idea 2)
    PRIMARY KEY (cs_ps1_fk, trade_direction)
);

CREATE TABLE asset_ters_score (
    asset_id TEXT NOT NULL, cs_ps1_fk BIGINT NOT NULL, trend_direction TEXT NOT NULL,
    oit_score DOUBLE PRECISION NOT NULL, fhr_score DOUBLE PRECISION NOT NULL,
    ters_score DOUBLE PRECISION NOT NULL, -- T.E.R.S. (Idea 1)
    PRIMARY KEY (cs_ps1_fk)
);

-- Final Output Score
CREATE TABLE asset_final_trade_confidence_score (
    asset_id TEXT NOT NULL,
    cs_ps1_fk BIGINT NOT NULL PRIMARY KEY,
    ucs_base_score DOUBLE PRECISION NOT NULL,
    reversal_factor DOUBLE PRECISION,
    efficacy_factor DOUBLE PRECISION,
    commitment_factor DOUBLE PRECISION,
    ftcs_score DOUBLE PRECISION NOT NULL, -- F.T.C.S.
    final_signal TEXT NOT NULL
);


------------------
-- ATR
-----------------
-- 1. CALCULATE ATR (M13 Dependency) and find T_break (Assumed available in asset_structural_takedowns)
-- ... (ATR Calculation and Data Gathering CTEs are included here as defined previously)
-- 2. CALCULATE BATTLE METRICS PER EVENT (T_end - T_start and Range)
WITH battle_calculation AS (
    SELECT
        rd.asset_id, rd.ps1_name, rd.ps1_bias_7, rd.cs_name, rd.prior_level,
        -- Duration (seconds): break_ts - cs_start_ts
        EXTRACT(EPOCH FROM (rd.break_ts - rd.cs_start_ts)) AS battle_duration_s,
        -- Price Range: (High - Low) normalized by Daily ATR
        (rd.cs_session_high - rd.cs_session_low) AS battle_price_range,
        rd.daily_atr
    FROM required_data rd
    WHERE rd.break_ts > rd.cs_start_ts
),
-- 3. AGGREGATE THE SBI BY PATTERN
aggregated_sbi AS (
    SELECT
        c.asset_id, c.ps1_name, c.ps1_bias_7, c.cs_name, c.prior_level,
        COUNT(*) AS total_breaks,
        AVG(c.battle_duration_s) AS avg_battle_duration_s,
        AVG(c.battle_price_range / c.daily_atr) * 100 AS avg_battle_range_pct
    FROM battle_calculation c GROUP BY 1, 2, 3, 4, 5
)
-- 4. FINAL INSERTION AND LABELING (M12)
INSERT INTO asset_structural_battle_index (
    ..., avg_battle_duration_s, avg_battle_range_pct, battle_label
)
SELECT
    ..., a.avg_battle_duration_s, a.avg_battle_range_pct,
    CASE
        WHEN a.avg_battle_duration_s < 900 AND a.avg_battle_range_pct < 10.0 THEN 'Decisive'
        ELSE 'Indecisive'
    END AS battle_label
FROM aggregated_sbi a ...

---------------------
--- OIT
-------------------

-- 1. IDENTIFY DIRECTIONAL MOVES AND PULLBACKS
WITH oit_events AS (
    SELECT
        c.asset_id, c.cs_ps1_fk, c.ps1_bias_7, c.cs_name, c.cs_direction AS trend_direction, c.cs_range,
        -- Calculate the counter-directional pullback size
        CASE
            WHEN c.cs_direction = 'BULLISH' THEN (c.high - c.close)
            WHEN c.cs_direction = 'BEARISH' THEN (c.close - c.low)
            ELSE 0.0
        END AS pullback_size,
        -- OIT Condition: Is Pullback > 50% of the Total CS Range?
        CASE
            WHEN (c.cs_direction = 'BULLISH' AND (c.high - c.close) > (c.cs_range * 0.50))
                 OR (c.cs_direction = 'BEARISH' AND (c.close - c.low) > (c.cs_range * 0.50)) THEN 1
            ELSE 0
        END AS is_oit_takedown
    FROM cs_directional_moves c WHERE c.cs_direction IN ('BULLISH', 'BEARISH')
),
-- 2. AGGREGATE RESULTS BY PATTERN
aggregated_oit AS (
    SELECT
        o.asset_id, o.ps1_bias_7, o.cs_name, o.trend_direction,
        COUNT(*) AS total_patterns,
        SUM(o.is_oit_takedown) AS oit_takedown_count,
        AVG(o.pullback_size / o.cs_range) * 100 AS avg_pullback_pct
    FROM oit_events o GROUP BY 1, 2, 3, 4
)
-- 3. FINAL INSERTION AND LABELING (M14)
INSERT INTO asset_open_interest_takedown (
    ..., oit_takedown_count, oit_score, avg_pullback_pct, oit_label
)
SELECT
    ..., a.oit_takedown_count, 
    (a.oit_takedown_count::DOUBLE PRECISION / a.total_patterns) AS oit_score,
    a.avg_pullback_pct,
    CASE
        WHEN (a.oit_takedown_count::DOUBLE PRECISION / a.total_patterns) >= 0.60 THEN 'Exhausted'
        ELSE 'Committed'
    END AS oit_label
FROM aggregated_oit a ...

-------------------
--- FCTS
------

-- 1. JOIN ALL SCORES (UCS, TERS, CBES, PDRS)
WITH all_scores AS (
    SELECT uc.ucs_score, ters.ters_score, cbes.cbes_score, pdrs.pdrs_score, ...
    FROM asset_unified_confidence_score uc
    LEFT JOIN asset_ters_score ters ON uc.cs_ps1_fk = ters.cs_ps1_fk
    LEFT JOIN asset_cbes_score cbes ON uc.cs_ps1_fk = cbes.cs_ps1_fk
    LEFT JOIN asset_pdrs_score pdrs ON uc.cs_ps1_fk = pdrs.cs_ps1_fk
),
-- 2. APPLY CONDITIONAL LOGIC FOR ADJUSTMENT FACTORS
factor_calculation AS (
    SELECT
        s.*,
        -- F_Reversal (TERS Penalty)
        CASE WHEN s.ters_score >= 0.65 THEN 0.50 WHEN s.ters_score > 0.40 THEN (1.0 - (s.ters_score / 1.30)) ELSE 1.0 END AS reversal_factor,
        -- F_Efficacy (CBES Boost/Penalty)
        CASE WHEN s.cbes_score >= 0.60 THEN 1.25 WHEN s.cbes_score > 0.30 THEN (s.cbes_score / 0.60) ELSE 0.50 END AS efficacy_factor,
        -- F_Commitment (PDRS Validation)
        CASE WHEN s.pdrs_score >= 1.20 THEN 1.20 WHEN s.pdrs_score > 0.75 THEN (s.pdrs_score / 1.0) ELSE 0.75 END AS commitment_factor
    FROM all_scores s
)
-- 3. FINAL CALCULATION AND INSERTION (F.T.C.S.)
INSERT INTO asset_final_trade_confidence_score (...)
SELECT
    f.asset_id, f.cs_ps1_fk, f.ucs_score * 100.0,
    f.reversal_factor, f.efficacy_factor, f.commitment_factor,
    -- FTCS = UCS * F_Rev * F_Eff * F_Com
    (f.ucs_score * f.reversal_factor * f.efficacy_factor * f.commitment_factor) AS ftcs_score,
    -- Final Signal Assignment
    CASE
        WHEN (f.ucs_score * f.reversal_factor * f.efficacy_factor * f.commitment_factor) >= 1.50 THEN 'Aggressive Entry (Extreme Confidence)'
        WHEN (f.ucs_score * f.reversal_factor * f.efficacy_factor * f.commitment_factor) >= 1.20 THEN 'Standard Entry (High Confidence)'
        WHEN (f.ucs_score * f.reversal_factor * f.efficacy_factor * f.commitment_factor) >= 0.90 THEN 'Caution/Wait (Moderate Confidence)'
        ELSE 'Avoid Trade (Low Confidence)'
    END AS final_signal
FROM factor_calculation f;