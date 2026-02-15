# Frozen Broker Clock Fix

## Problem
On Friday market close, the MT5 broker's internal clock freezes. This causes:
- By Sunday: 36+ hour time drift
- Database receives wrong UTC timestamps
- Offset calibration fails

## Solution Implemented

### Rust Side (shared_models/src/data_model.rs)
Added **three-layer frozen clock detection**:

1. **Raw Offset Check** (line ~175)
   - If `abs(raw_offset) > 43200 seconds` (12 hours), clock is likely frozen
   - Rejects the bad calibration immediately, even on first attempt
   - Falls back to last known-good offset

2. **Broker Time Stalled Check** (line ~195)
   - If `broker_time` hasn't changed since last calibration, clock is frozen
   - Uses fallback offset

3. **Excessive Drift Check** (line ~205)
   - If offset changes by > 6 hours, rejects the bad reading
   - Uses fallback offset

### MQL5 Side (Need to Update)
Currently sends calibration **once on connect** and **once per hour**.
**This is too infrequent** - by the time Sunday arrives, there's no recent valid calibration to fall back to.

**TODO: Update pipe_data.mq5 to calibrate every 5-10 minutes instead of every hour**

Example change:
```cpp
// OLD: static uint last_cal_ticks = 0;
//      if(last_cal_time == 0 || ticks - last_cal_ticks > 3600000) { // 1 hour

// NEW: Send calibration every 5 minutes to catch freeze early
if(last_cal_time == 0 || ticks - last_cal_ticks > 300000) { // 5 minutes (300,000 ms)
   CalibrationPacket cal;
   cal.broker_time = TimeCurrent();
   cal.gmt_time    = TimeGMT();
   SafeSend(254, cal);
   last_cal_ticks = ticks;
   last_cal_time = cal.broker_time; 
}
```

## How It Works on Sunday Now

1. **Friday evening**: Clock freezes at +2 GMT ✅ Calibration valid
2. **Saturday morning**: MT5 clock still frozen, but Rust rejects it ✅ Uses Friday's +2 offset
3. **Sunday open**: Clock still frozen, large offset detected ✅ Falls back to +2
4. **First live tick Sunday**: Correctly offsets to UTC using +2 ✅

## Key Changes Made

| Component | Change | Reason |
|-----------|--------|--------|
| `CalibrationState` struct | Added to track last valid offset | Enables fallback when freeze detected |
| `handle_calibration()` | Added 3-layer detection | Catches frozen clock immediately |
| Offset validation | Check `abs(raw_offset) > 43200` | Freeze = offset too large |
| Frequency (TODO) | Increase to 5-min intervals | Ensures recent valid calibration exists |

## Testing

Run with broker frozen and check logs:
```
⚠️  FROZEN CLOCK DETECTED: Raw offset ...s (...h) is too large!
🔒 Using fallback offset from last valid calibration: GMT+2
```

This should appear instead of incorrectly showing GMT+9.
