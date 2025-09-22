use  anyhow::{Result, Context} ;
use crate::DB;

pub async fn candle_pattern_function(db: &DB) -> Result<()> {
    println!("  - Defining Candle Pattern function...");
   let candle_pattern_function = r#"
        DEFINE FUNCTION fn::candle_pattern($open: float, $high: float, $low: float, $close: float) -> string {
            LET $doji_body_ratio = 0.1;
            LET $body_wick_ratio_long = 0.5;
            LET $body_wick_ratio_short = 0.3;
            LET $upper_vs_lower_ratio = 0.6;
            LET $eps = 0.000000001;
            LET $full_range = $high - $low;
            LET $body_range = math::abs($close - $open);

            IF $full_range < $eps THEN
                RETURN 'Unknown';
            END;

            LET $upper_wick = $high - math::max($close, $open);
            LET $lower_wick = math::min($open, $close) - $low;

            LET $body_ratio = $body_range / $full_range;
            LET $upper_wick_ratio = $upper_wick / $full_range;
            LET $lower_wick_ratio = $lower_wick / $full_range;

            LET $is_bullish = $close > $open;

            IF $body_ratio <= $doji_body_ratio THEN
                RETURN 'Doji/SpinningTop';
            END;

            IF $body_ratio < $body_wick_ratio_short THEN
                IF $upper_wick_ratio / ($lower_wick_ratio + $eps) < $upper_vs_lower_ratio THEN
                    IF $is_bullish THEN
                        RETURN 'Bullish Hammer';
                    ELSE
                        RETURN 'Bearish Hammer';
                    END;
                ELSE
                    IF $lower_wick_ratio / ($upper_wick_ratio + $eps) < $upper_vs_lower_ratio THEN
                        IF $is_bullish THEN
                            RETURN 'Bullish Shooting Star';
                        ELSE
                            RETURN 'Bearish Shooting Star';
                        END;
                    END;
                END;
            END;

            IF $body_ratio >= $body_wick_ratio_long THEN
                IF $is_bullish THEN
                    RETURN 'Bullish Long Body';
                ELSE
                    RETURN 'Bearish Long Body';
                END;
            END;

            IF $is_bullish THEN
                RETURN 'Mild Bullish';
            ELSE
                RETURN 'Mild Bearish';
            END;
};
    "#;

    db.query(candle_pattern_function)
        .await
        .context("Failed to define Candle Pattern function")?;

    println!("  - Candle Pattern function defined. ✅");
    Ok(())
}