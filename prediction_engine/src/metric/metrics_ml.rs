use std::collections::HashMap;
use chrono::{NaiveDate, Datelike};
use anyhow::Result;

// 1. Alias the modules to keep the code clean and scannable
use shared_models as sm;
use crate::metric::metrics_model as mm;
use crate::metric::metrics_context as ctx;
use crate::metric::metrics_service::MetricsService;

impl MetricsService {

    pub async fn calculate_cs_base_rates_ml(
        &self,
        session_contexts: &[sm::SessionContextData],
        intervals: &[sm::LookbackInterval],
    ) -> Result<Vec<mm::SessionBaseRateML>> {
        let base_contexts: Vec<ctx::BaseRateSessionContext> = session_contexts.iter()
            .cloned()
            .map(|s| ctx::BaseRateSessionContext { inner: s })
            .collect();

        let (counts, totals) = self.aggregate_ml_features(&base_contexts, intervals);

        let cs_base_rates = counts.into_iter()
            .map(|(key, interval_counts)| {
                let (asset_id, cs_name, cs_bias) = key;
                let total_key = (asset_id.clone(), cs_name.clone());
                let interval_totals = totals.get(&total_key);

                let get_stats = |name: &str| {
                    let count = *interval_counts.get(name).unwrap_or(&0.0);
                    let total = interval_totals.and_then(|t| t.get(name)).cloned().unwrap_or(0.0);
                    let prob = if total > 0.0 { count / total } else { 0.0 };
                    (prob, count, total)
                };

                let (p_6m, c_6m, t_6m) = get_stats("6M");
                let (p_1y, c_1y, t_1y) = get_stats("1Y");
                let (p_all, c_all, t_all) = get_stats("ALL");

                mm::SessionBaseRateML {
                    asset_id,
                    session_name: cs_name,
                    session_bias: cs_bias,
                    p_6m, count_6m: c_6m, total_6m: t_6m,
                    p_1y, count_1y: c_1y, total_1y: t_1y,
                    p_all, count_all: c_all, total_all: t_all,
                }
            })
            .collect();

        Ok(cs_base_rates)
    }

    pub async fn calculate_bar_base_rates_ml(
        &self,
        eight_contexts: &[sm::EightContextData],
        intervals: &[sm::LookbackInterval],
    ) -> Result<Vec<mm::BarBaseRateML>> {
        let bar_contexts: Vec<ctx::BarBaseRateContext> = eight_contexts.iter()
            .cloned()
            .map(|s| ctx::BarBaseRateContext { inner: s })
            .collect();

        let (counts, totals) = self.aggregate_ml_features(&bar_contexts, intervals);

        let bar_base_rates = counts.into_iter()
            .map(|(key, interval_counts)| {
                let (asset_id, bar_num, bar_bias) = key;
                let total_key = (asset_id.clone(), bar_num);
                let interval_totals = totals.get(&total_key);

                let get_stats = |name: &str| {
                    let count = *interval_counts.get(name).unwrap_or(&0.0);
                    let total = interval_totals.and_then(|t| t.get(name)).cloned().unwrap_or(0.0);
                    let prob = if total > 0.0 { count / total } else { 0.0 };
                    (prob, count, total)
                };

                let (p_6m, c_6m, t_6m) = get_stats("6M");
                let (p_1y, c_1y, t_1y) = get_stats("1Y");
                let (p_all, c_all, t_all) = get_stats("ALL");

                mm::BarBaseRateML {
                    asset_id, bar_num, bar_bias,
                    p_6m, count_6m: c_6m, total_6m: t_6m,
                    p_1y, count_1y: c_1y, total_1y: t_1y,
                    p_all, count_all: c_all, total_all: t_all,
                }
            })
            .collect();

        Ok(bar_base_rates)
    }

    pub async fn calculate_daily_base_rates_ml(
        &self,
        daily_data: &[sm::DailyContextData],
        intervals: &[sm::LookbackInterval],
    ) -> Result<Vec<mm::DailyBaseRateML>> {
        let contexts: Vec<ctx::DailyBaseRateContext> = daily_data.iter()
            .cloned()
            .map(|d| ctx::DailyBaseRateContext { inner: d })
            .collect();

        let (counts, totals) = self.aggregate_ml_features(&contexts, intervals);

        let daily_base_rates = counts.into_iter()
            .map(|(key, interval_counts)| {
                let (asset_id, _, daily_bias) = key;
                let total_key = (asset_id.clone(), asset_id.clone());
                let interval_totals = totals.get(&total_key);

                let get_stats = |name: &str| {
                    let count = *interval_counts.get(name).unwrap_or(&0.0);
                    let total = interval_totals.and_then(|t| t.get(name)).cloned().unwrap_or(0.0);
                    let prob = if total > 0.0 { count / total } else { 0.0 };
                    (prob, count, total)
                };

                let (p_6m, c_6m, t_6m) = get_stats("6M");
                let (p_1y, c_1y, t_1y) = get_stats("1Y");
                let (p_all, c_all, t_all) = get_stats("ALL");

                mm::DailyBaseRateML {
                    asset_id, daily_bias,
                    p_6m, count_6m: c_6m, total_6m: t_6m,
                    p_1y, count_1y: c_1y, total_1y: t_1y,
                    p_all, count_all: c_all, total_all: t_all,
                }
            })
            .collect();

        Ok(daily_base_rates)
    }

    pub async fn calculate_transition_2nd_order_ml(
        &self,
        session_contexts: &[sm::SessionContextData],
        intervals: &[sm::LookbackInterval],
    ) -> Result<Vec<mm::Transition2ndOrderML>> {
        let contexts: Vec<ctx::Transition2ndOrderContext> = session_contexts.iter()
            .cloned()
            .map(|s| ctx::Transition2ndOrderContext { inner: s })
            .collect();

        let (counts, totals) = self.aggregate_ml_features(&contexts, intervals);

        let results = counts.into_iter().map(|(key, interval_counts)| {
            let (asset_id, (ps2n, ps2b, ps1n, ps1b), (csn, csb)) = key;
            let total_key = (asset_id.clone(), (ps2n.clone(), ps2b.clone(), ps1n.clone(), ps1b.clone()));
            let interval_totals = totals.get(&total_key);

            let get_stats = |name: &str| {
                let count = *interval_counts.get(name).unwrap_or(&0.0);
                let total = interval_totals.and_then(|t| t.get(name)).cloned().unwrap_or(0.0);
                let prob = if total > 0.0 { count / total } else { 0.0 };
                (prob, count, total)
            };

            let (p_6m, c_6m, t_6m) = get_stats("6M");
            let (p_1y, c_1y, t_1y) = get_stats("1Y");
            let (p_all, c_all, t_all) = get_stats("ALL");

            mm::Transition2ndOrderML {
                asset_id, ps2_name: ps2n, ps2_bias: ps2b, ps1_name: ps1n, ps1_bias: ps1b, cs_name: csn, cs_bias: csb,
                p_6m, count_6m: c_6m, total_6m: t_6m,
                p_1y, count_1y: c_1y, total_1y: t_1y,
                p_all, count_all: c_all, total_all: t_all,
            }
        }).collect();
        Ok(results)
    }

    pub async fn calculate_tcs_scores_ml(
        &self,
        transitions: Vec<mm::Transition2ndOrderML>,
        base_rates: Vec<mm::SessionBaseRateML>,
    ) -> Result<Vec<mm::Tcs2ndOrderML>> {
        let base_map: HashMap<(String, String, String), mm::SessionBaseRateML> = base_rates.into_iter()
            .map(|r| ((r.asset_id.clone(), r.session_name.clone(), r.session_bias.clone()), r))
            .collect();

        let scores = transitions.into_iter().filter_map(|t| {
            let br = base_map.get(&(t.asset_id.clone(), t.cs_name.clone(), t.cs_bias.clone()))?;

            let calc = |p_cond: f64, p_base: f64| {
                let score = if p_base > 0.0 { p_cond / p_base } else { 1.0 };
                (p_cond, p_base, score)
            };

            let (pc_6m, pb_6m, s_6m) = calc(t.p_6m, br.p_6m);
            let (pc_1y, pb_1y, s_1y) = calc(t.p_1y, br.p_1y);
            let (pc_all, pb_all, s_all) = calc(t.p_all, br.p_all);

            Some(mm::Tcs2ndOrderML {
                asset_id: t.asset_id, ps2_name: t.ps2_name, ps2_bias: t.ps2_bias, 
                ps1_name: t.ps1_name, ps1_bias: t.ps1_bias, cs_name: t.cs_name, cs_bias: t.cs_bias,
                p_cond_6m: pc_6m, p_base_6m: pb_6m, tcs_score_6m: s_6m,
                p_cond_1y: pc_1y, p_base_1y: pb_1y, tcs_score_1y: s_1y,
                p_cond_all: pc_all, p_base_all: pb_all, tcs_score_all: s_all,
            })
        }).collect();
        Ok(scores)
    }

    pub async fn calculate_bar_transition_2nd_order_ml(
        &self,
        eight_contexts: &[sm::EightContextData],
        intervals: &[sm::LookbackInterval],
    ) -> Result<Vec<mm::BarTransition2ndOrderML>> {
        let contexts: Vec<ctx::Transition2ndOrderBarContext> = eight_contexts.iter()
            .cloned()
            .map(|s| ctx::Transition2ndOrderBarContext { inner: s })
            .collect();

        let (counts, totals) = self.aggregate_ml_features(&contexts, intervals);

        let results = counts.into_iter().map(|(key, interval_counts)| {
            let (asset_id, (pb2n, pb2b, pb1n, pb1b), (cbn, cbb)) = key;
            let total_key = (asset_id.clone(), (pb2n, pb2b.clone(), pb1n, pb1b.clone()));
            let interval_totals = totals.get(&total_key);

            let get_stats = |name: &str| {
                let count = *interval_counts.get(name).unwrap_or(&0.0);
                let total = interval_totals.and_then(|t| t.get(name)).cloned().unwrap_or(0.0);
                let prob = if total > 0.0 { count / total } else { 0.0 };
                (prob, count, total)
            };

            let (p_6m, c_6m, t_6m) = get_stats("6M");
            let (p_1y, c_1y, t_1y) = get_stats("1Y");
            let (p_all, c_all, t_all) = get_stats("ALL");

            mm::BarTransition2ndOrderML {
                asset_id, pb2_num: pb2n, pb2_bias: pb2b, pb1_num: pb1n, pb1_bias: pb1b, cb_num: cbn, cb_bias: cbb,
                p_6m, count_6m: c_6m, total_6m: t_6m,
                p_1y, count_1y: c_1y, total_1y: t_1y,
                p_all, count_all: c_all, total_all: t_all,
            }
        }).collect();
        Ok(results)
    }

    pub async fn calculate_stcs_scores_ml(
        &self,
        transitions: Vec<mm::BarTransition2ndOrderML>,
        bar_base_rates: Vec<mm::BarBaseRateML>,
    ) -> Result<Vec<mm::Stcs2ndOrderML>> {
        let base_map: HashMap<(String, i32, String), mm::BarBaseRateML> = bar_base_rates.into_iter()
            .map(|r| ((r.asset_id.clone(), r.bar_num, r.bar_bias.clone()), r))
            .collect();

        let scores = transitions.into_iter().filter_map(|t| {
            let br = base_map.get(&(t.asset_id.clone(), t.cb_num, t.cb_bias.clone()))?;

            let calc = |p_cond: f64, p_base: f64| {
                let score = if p_base > 0.0 { p_cond / p_base } else { 1.0 };
                (p_cond, p_base, score)
            };

            let (pc_6m, pb_6m, s_6m) = calc(t.p_6m, br.p_6m);
            let (pc_1y, pb_1y, s_1y) = calc(t.p_1y, br.p_1y);
            let (pc_all, pb_all, s_all) = calc(t.p_all, br.p_all);

            Some(mm::Stcs2ndOrderML {
                asset_id: t.asset_id, pb2_num: t.pb2_num, pb2_bias: t.pb2_bias,
                pb1_num: t.pb1_num, pb1_bias: t.pb1_bias, cb_num: t.cb_num, cb_bias: t.cb_bias,
                p_cond_6m: pc_6m, p_base_6m: pb_6m, stcs_score_6m: s_6m,
                p_cond_1y: pc_1y, p_base_1y: pb_1y, stcs_score_1y: s_1y,
                p_cond_all: pc_all, p_base_all: pb_all, stcs_score_all: s_all,
            })
        }).collect();
        Ok(scores)
    }

    pub async fn calculate_bar_to_daily_ml(
        &self,
        eight_data: &[sm::EightContextData],
        daily_outcomes: &HashMap<NaiveDate, sm::DailyContextData>,
        intervals: &[sm::LookbackInterval],
    ) -> Result<Vec<mm::BarToDailyOutcomeML>> {
        let contexts: Vec<ctx::DayType2ndOrderBarContext> = eight_data.iter()
            .filter_map(|e| {
                daily_outcomes.get(&e.trading_date).map(|d| ctx::DayType2ndOrderBarContext {
                    inner: e.clone(),
                    outcome: d.clone(),
                })
            })
            .collect();

        let (counts, totals) = self.aggregate_ml_features(&contexts, intervals);

        let results = counts.into_iter().map(|(key, interval_counts)| {
            let (asset_id, (pb2n, pb2b, pb1n, pb1b), day_type) = key;
            let total_key = (asset_id.clone(), (pb2n, pb2b.clone(), pb1n, pb1b.clone()));
            let interval_totals = totals.get(&total_key);

            let get_stats = |name: &str| {
                let count = *interval_counts.get(name).unwrap_or(&0.0);
                let total = interval_totals.and_then(|t| t.get(name)).cloned().unwrap_or(0.0);
                let prob = if total > 0.0 { count / total } else { 0.0 };
                (prob, count, total)
            };

            let (p_6m, c_6m, t_6m) = get_stats("6M");
            let (p_1y, c_1y, t_1y) = get_stats("1Y");
            let (p_all, c_all, t_all) = get_stats("ALL");

            mm::BarToDailyOutcomeML {
                asset_id, pb2_num: pb2n, pb2_bias: pb2b, pb1_num: pb1n, pb1_bias: pb1b, day_type,
                p_6m, count_6m: c_6m, total_6m: t_6m,
                p_1y, count_1y: c_1y, total_1y: t_1y,
                p_all, count_all: c_all, total_all: t_all,
            }
        }).collect();
        Ok(results)
    }

    pub async fn calculate_bar_to_daily_lift_ml(
        &self,
        transitions: Vec<mm::BarToDailyOutcomeML>,
        daily_base_rates: Vec<mm::DailyBaseRateML>,
    ) -> Result<Vec<mm::BarToDailyLiftML>> {
        let base_map: HashMap<(String, String), mm::DailyBaseRateML> = daily_base_rates.into_iter()
            .map(|r| ((r.asset_id.clone(), r.daily_bias.clone()), r))
            .collect();

        let lift_results = transitions.into_iter().filter_map(|t| {
            let br = base_map.get(&(t.asset_id.clone(), t.day_type.clone()))?;

            let calc_lift = |p_cond: f64, p_base: f64| {
                let lift = if p_base > 0.0 { p_cond / p_base } else { 1.0 };
                (p_cond, p_base, lift)
            };

            let (pc_6m, pb_6m, l_6m) = calc_lift(t.p_6m, br.p_6m);
            let (pc_1y, pb_1y, l_1y) = calc_lift(t.p_1y, br.p_1y);
            let (pc_all, pb_all, l_all) = calc_lift(t.p_all, br.p_all);

            Some(mm::BarToDailyLiftML {
                asset_id: t.asset_id,
                pb2_num: t.pb2_num, pb2_bias: t.pb2_bias,
                pb1_num: t.pb1_num, pb1_bias: t.pb1_bias,
                day_type: t.day_type,
                p_cond_6m: pc_6m, p_base_6m: pb_6m, lift_6m: l_6m,
                p_cond_1y: pc_1y, p_base_1y: pb_1y, lift_1y: l_1y,
                p_cond_all: pc_all, p_base_all: pb_all, lift_all: l_all,
            })
        }).collect();

        Ok(lift_results)
    }

    pub async fn calculate_session_to_daily_ml(
        &self,
        session_data: &[sm::SessionContextData],
        daily_outcomes: &HashMap<NaiveDate, sm::DailyContextData>,
        intervals: &[sm::LookbackInterval],
    ) -> Result<Vec<mm::SessionToDailyOutcomeML>> {
        let contexts: Vec<ctx::DayType2ndOrderSessionContext> = session_data.iter()
            .filter_map(|s| {
                daily_outcomes.get(&s.trading_date).map(|d| ctx::DayType2ndOrderSessionContext {
                    inner: s.clone(),
                    outcome: d.clone(),
                })
            })
            .collect();

        let (counts, totals) = self.aggregate_ml_features(&contexts, intervals);

        let results = counts.into_iter().map(|(key, interval_counts)| {
            let (asset_id, (ps2n, ps2b, ps1n, ps1b), day_type) = key;
            let total_key = (asset_id.clone(), (ps2n.clone(), ps2b.clone(), ps1n.clone(), ps1b.clone()));
            let interval_totals = totals.get(&total_key);

            let get_stats = |name: &str| {
                let count = *interval_counts.get(name).unwrap_or(&0.0);
                let total = interval_totals.and_then(|t| t.get(name)).cloned().unwrap_or(0.0);
                let prob = if total > 0.0 { count / total } else { 0.0 };
                (prob, count, total)
            };

            let (p_6m, c_6m, t_6m) = get_stats("6M");
            let (p_1y, c_1y, t_1y) = get_stats("1Y");
            let (p_all, c_all, t_all) = get_stats("ALL");

            mm::SessionToDailyOutcomeML {
                asset_id, ps2_name: ps2n, ps2_bias: ps2b, ps1_name: ps1n, ps1_bias: ps1b, day_type,
                p_6m, count_6m: c_6m, total_6m: t_6m,
                p_1y, count_1y: c_1y, total_1y: t_1y,
                p_all, count_all: c_all, total_all: t_all,
            }
        }).collect();
        Ok(results)
    }

    pub async fn calculate_session_to_daily_lift_ml(
        &self,
        transitions: Vec<mm::SessionToDailyOutcomeML>,
        daily_base_rates: Vec<mm::DailyBaseRateML>,
    ) -> Result<Vec<mm::SessionToDailyLiftML>> {
        let base_map: HashMap<(String, String), mm::DailyBaseRateML> = daily_base_rates.into_iter()
            .map(|r| ((r.asset_id.clone(), r.daily_bias.clone()), r))
            .collect();

        let lift_results = transitions.into_iter().filter_map(|t| {
            let br = base_map.get(&(t.asset_id.clone(), t.day_type.clone()))?;

            let calc_lift = |p_cond: f64, p_base: f64| {
                let lift = if p_base > 0.0 { p_cond / p_base } else { 1.0 };
                (p_cond, p_base, lift)
            };

            let (pc_6m, pb_6m, l_6m) = calc_lift(t.p_6m, br.p_6m);
            let (pc_1y, pb_1y, l_1y) = calc_lift(t.p_1y, br.p_1y);
            let (pc_all, pb_all, l_all) = calc_lift(t.p_all, br.p_all);

            Some(mm::SessionToDailyLiftML {
                asset_id: t.asset_id,
                ps2_name: t.ps2_name, ps2_bias: t.ps2_bias,
                ps1_name: t.ps1_name, ps1_bias: t.ps1_bias,
                day_type: t.day_type,
                p_cond_6m: pc_6m, p_base_6m: pb_6m, lift_6m: l_6m,
                p_cond_1y: pc_1y, p_base_1y: pb_1y, lift_1y: l_1y,
                p_cond_all: pc_all, p_base_all: pb_all, lift_all: l_all,
            })
        }).collect();

        Ok(lift_results)
    }

    pub async fn calculate_session_to_bar_ml(
        &self,
        session_contexts: &[sm::SessionContextData],
        eight_hour_contexts: &[sm::EightContextData],
        intervals: &[sm::LookbackInterval],
    ) -> Result<Vec<mm::SessionToBarOutcomeML>> {
        let mut bar_map: HashMap<String, Vec<sm::EightContextData>> = HashMap::new();
        for b in eight_hour_contexts {
            bar_map.entry(b.asset_id.clone()).or_default().push(b.clone());
        }

        let joined_contexts: Vec<ctx::BarType2ndOrderSessionContext> = session_contexts.iter()
            .filter_map(|s| {
                let asset_bars = bar_map.get(&s.asset_id)?;
                let target_num = match s.ps1_name.as_deref() {
                    Some("NYPM") => 1,
                    Some("AS")   => 2,
                    Some("LN") | Some("NYAM") | Some("NYL") => 3,
                    _ => return None,
                };

                let target = asset_bars.iter()
                    .filter(|b| b.cb_num == Some(target_num))
                    .find(|b| b.session_end_ts > s.session_end_ts)?;

                Some(ctx::BarType2ndOrderSessionContext {
                    inner: s.clone(),
                    bar_outcome: target.clone(),
                })
            })
            .collect();

        let (counts, totals) = self.aggregate_ml_features(&joined_contexts, intervals);

        let results = counts.into_iter().map(|(key, interval_counts)| {
            let (asset_id, (ps2n, ps2b, ps1n, ps1b), (cb_num, cb_bias)) = key;
            let total_key = (asset_id.clone(), (ps2n.clone(), ps2b.clone(), ps1n.clone(), ps1b.clone()));
            let interval_totals = totals.get(&total_key);

            let get_stats = |name: &str| {
                let count = *interval_counts.get(name).unwrap_or(&0.0);
                let total = interval_totals.and_then(|t| t.get(name)).cloned().unwrap_or(0.0);
                let prob = if total > 0.0 { count / total } else { 0.0 };
                (prob, count, total)
            };

            let (p_6m, c_6m, t_6m) = get_stats("6M");
            let (p_1y, c_1y, t_1y) = get_stats("1Y");
            let (p_all, c_all, t_all) = get_stats("ALL");

            mm::SessionToBarOutcomeML {
                asset_id, ps2_name: ps2n, ps2_bias: ps2b, ps1_name: ps1n, ps1_bias: ps1b,
                cb_num, cb_bias,
                p_6m, count_6m: c_6m, total_6m: t_6m,
                p_1y, count_1y: c_1y, total_1y: t_1y,
                p_all, count_all: c_all, total_all: t_all,
            }
        }).collect();

        Ok(results)
    }

    pub async fn calculate_session_to_bar_lift_ml(
        &self,
        outcomes: Vec<mm::SessionToBarOutcomeML>,
        bar_base_rates: Vec<mm::BarBaseRateML>,
    ) -> Result<Vec<mm::SessionToBarLiftML>> {
        let base_map: HashMap<(String, i32, String), mm::BarBaseRateML> = bar_base_rates.into_iter()
            .map(|r| ((r.asset_id.clone(), r.bar_num, r.bar_bias.clone()), r))
            .collect();

        let lifts = outcomes.into_iter().filter_map(|o| {
            let br = base_map.get(&(o.asset_id.clone(), o.cb_num, o.cb_bias.clone()))?;

            let calc_lift = |p_cond: f64, p_base: f64| {
                let lift = if p_base > 0.0 { p_cond / p_base } else { 1.0 };
                (p_cond, p_base, lift)
            };

            let (pc_6m, pb_6m, l_6m) = calc_lift(o.p_6m, br.p_6m);
            let (pc_1y, pb_1y, l_1y) = calc_lift(o.p_1y, br.p_1y);
            let (pc_all, pb_all, l_all) = calc_lift(o.p_all, br.p_all);

            Some(mm::SessionToBarLiftML {
                asset_id: o.asset_id,
                ps2_name: o.ps2_name, ps2_bias: o.ps2_bias,
                ps1_name: o.ps1_name, ps1_bias: o.ps1_bias,
                cb_num: o.cb_num, cb_bias: o.cb_bias,
                p_cond_6m: pc_6m, p_base_6m: pb_6m, lift_6m: l_6m,
                p_cond_1y: pc_1y, p_base_1y: pb_1y, lift_1y: l_1y,
                p_cond_all: pc_all, p_base_all: pb_all, lift_all: l_all,
            })
        }).collect();

        Ok(lifts)
    }

    pub async fn calculate_daily_3rd_order_ml(
        &self,
        daily_raw: &[sm::DailyContextData], // We take the raw daily data to build the window
        intervals: &[sm::LookbackInterval],
    ) -> Result<Vec<mm::DailyOutcome3rdOrderML>> {
        
        // 1. Build the sliding window contexts (PD2 -> PD1 -> Current)
        let mut prepared_contexts: Vec<ctx::Daily3rdOrderContext> = Vec::new();
        
        // We need at least 3 days to form a 3rd order sequence
        for i in 2..daily_raw.len() {
            let current = &daily_raw[i];
            let pd1 = &daily_raw[i - 1]; 
            let pd2 = &daily_raw[i - 2]; 
            
            // weekday() returns a Weekday enum, we convert to string only when pushing
            let pd1_dow = pd1.trading_date.weekday().to_string(); 

            prepared_contexts.push(ctx::Daily3rdOrderContext {
                trading_date: current.trading_date,
                asset_id: current.asset_id.clone(),
                pd2_bias: pd2.day_type.clone(),
                pd1_bias: pd1.day_type.clone(),
                pd1_dow: Some(pd1_dow), 
                c_day_bias: current.day_type.clone(),
            });
        }

        if prepared_contexts.is_empty() {
            return Ok(Vec::new());
        }

        // 2. Aggregate the features using the prepared sliding-window contexts
        let (counts, totals) = self.aggregate_ml_features(&prepared_contexts, intervals);

        // 3. Map to the ML Wide Struct
        let results = counts.into_iter().map(|(key, interval_counts)| {
            let (asset_id, (pd2, pd1, dow), day_type) = key;
            
            let total_key = (asset_id.clone(), (pd2.clone(), pd1.clone(), dow.clone()));
            let interval_totals = totals.get(&total_key);

            let get_stats = |name: &str| {
                let count = *interval_counts.get(name).unwrap_or(&0.0);
                let total = interval_totals.and_then(|t| t.get(name)).cloned().unwrap_or(0.0);
                let prob = if total > 0.0 { count / total } else { 0.0 };
                (prob, count, total)
            };

            let (p_6m, c_6m, t_6m) = get_stats("6M");
            let (p_1y, c_1y, t_1y) = get_stats("1Y");
            let (p_all, c_all, t_all) = get_stats("ALL");

            mm::DailyOutcome3rdOrderML {
                asset_id, pd2_bias: pd2, pd1_bias: pd1, pd1_dow: dow, day_type,
                p_6m, count_6m: c_6m, total_6m: t_6m,
                p_1y, count_1y: c_1y, total_1y: t_1y,
                p_all, count_all: c_all, total_all: t_all,
            }
        }).collect();

        Ok(results)
    }

    pub async fn calculate_daily_3rd_order_lift_ml(
        &self,
        outcomes: Vec<mm::DailyOutcome3rdOrderML>,
        daily_base_rates: Vec<mm::DailyBaseRateML>,
    ) -> Result<Vec<mm::DailyOutcome3rdOrderLiftML>> {
        
        let base_map: HashMap<(String, String), mm::DailyBaseRateML> = daily_base_rates.into_iter()
            .map(|r| ((r.asset_id.clone(), r.daily_bias.clone()), r))
            .collect();

        let lifts = outcomes.into_iter().filter_map(|o| {
            let br = base_map.get(&(o.asset_id.clone(), o.day_type.clone()))?;

            let calc_lift = |pc: f64, pb: f64| (pc, pb, if pb > 0.0 { pc / pb } else { 1.0 });

            let (p6c, p6b, l6) = calc_lift(o.p_6m, br.p_6m);
            let (p1c, p1b, l1) = calc_lift(o.p_1y, br.p_1y);
            let (pac, pab, la) = calc_lift(o.p_all, br.p_all);

            Some(mm::DailyOutcome3rdOrderLiftML {
                asset_id: o.asset_id, pd2_bias: o.pd2_bias, pd1_bias: o.pd1_bias, pd1_dow: o.pd1_dow, day_type: o.day_type,
                p_cond_6m: p6c, p_base_6m: p6b, lift_6m: l6,
                p_cond_1y: p1c, p_base_1y: p1b, lift_1y: l1,
                p_cond_all: pac, p_base_all: pab, lift_all: la,
            })
        }).collect();

        Ok(lifts)   
    }

    pub async fn calculate_session_continuation_ml(
        &self,
        session_data: &[sm::SessionContextData],
        intervals: &[sm::LookbackInterval],
    ) -> Result<Vec<mm::SessionContinuationML>> {
        
        let contexts: Vec<ctx::SessionContinuationContext> = session_data.iter()
            .cloned()
            .map(|s| ctx::SessionContinuationContext { inner: s })
            .collect();

        let (counts, totals) = self.aggregate_ml_features(&contexts, intervals);

        let results = self.extract_continuation_results(counts, totals, 
            |asset_id, (ps2n, ps2b, ps1n, ps1b, csn), s6, s1, sa| {
                mm::SessionContinuationML {
                    asset_id,
                    ps2_name: ps2n, ps2_bias: ps2b,
                    ps1_name: ps1n, ps1_bias: ps1b,
                    cs_name: csn,
                    p_6m: s6.0, count_6m: s6.1, total_6m: s6.2,
                    p_1y: s1.0, count_1y: s1.1, total_1y: s1.2,
                    p_all: sa.0, count_all: sa.1, total_all: sa.2,
                }
        });

        Ok(results)
    }

    pub async fn calculate_bar_continuation_ml(
        &self,
        bar_data: &[sm::EightContextData],
        intervals: &[sm::LookbackInterval],
    ) -> Result<Vec<mm::BarContinuationML>> {
        
        // No sliding window needed! Just wrap the existing data.
        let contexts: Vec<ctx::BarContinuationContext> = bar_data.iter()
            .map(|b| ctx::BarContinuationContext { inner: b.clone() })
            .collect();

        let (counts, totals) = self.aggregate_ml_features(&contexts, intervals);

        let results = self.extract_continuation_results(counts, totals, 
            |asset_id, (pb2n, pb2b, pb1n, pb1b, cbn), s6, s1, sa| {
                mm::BarContinuationML {
                    asset_id,
                    pb2_num: pb2n, pb2_bias: pb2b,
                    pb1_num: pb1n, pb1_bias: pb1b,
                    cb_num: cbn,
                    p_6m: s6.0, count_6m: s6.1, total_6m: s6.2,
                    p_1y: s1.0, count_1y: s1.1, total_1y: s1.2,
                    p_all: sa.0, count_all: sa.1, total_all: sa.2,
                }
        });

        Ok(results)
    }

    pub async fn calculate_daily_continuation_ml(
        &self,
        daily_raw: &[sm::DailyContextData],
        intervals: &[sm::LookbackInterval],
    ) -> Result<Vec<mm::DailyContinuationML>> {
        let mut contexts = Vec::new();

        // Sliding window for Days (PD2 -> PD1 -> CD)
        for i in 2..daily_raw.len() {
            contexts.push(ctx::DailyContinuationContext {
                pd2: daily_raw[i - 2].clone(),
                pd1: daily_raw[i - 1].clone(),
                cd: daily_raw[i].clone(),
            });
        }

        let (counts, totals) = self.aggregate_ml_features(&contexts, intervals);

        let results = self.extract_continuation_results(counts, totals, 
            |asset_id, (pd2b, pd1b, dow), s6, s1, sa| {
                mm::DailyContinuationML {
                    asset_id,
                    pd2_bias: pd2b, pd1_bias: pd1b, pd1_dow: dow,
                    p_6m: s6.0, count_6m: s6.1, total_6m: s6.2,
                    p_1y: s1.0, count_1y: s1.1, total_1y: s1.2,
                    p_all: sa.0, count_all: sa.1, total_all: sa.2,
                }
        });
        Ok(results)
    }


    /// Session Exterme Events ML Calculations
    
    pub async fn calculate_session_extreme_base_rates_ml(
        &self,
        daily_raw: &[sm::DailyContextData],
        intervals: &[sm::LookbackInterval],
    ) -> Result<Vec<mm::SessionExtremeBaseRateML>> {
        
        // Use flat_map instead of filter_map since we know the strings exist, 
        // but we want to skip "Other" or empty values.
        let contexts: Vec<ctx::SessionExtremeBaseContext> = daily_raw.iter().flat_map(|d| {
            let mut daily_vec = Vec::new();

            // Check High Session
            if !d.high_session.is_empty() && d.high_session != "Other" {
                daily_vec.push(ctx::SessionExtremeBaseContext {
                    trading_date: d.trading_date,
                    asset_id: d.asset_id.clone(),
                    extreme_type: "High".to_string(),
                    extreme_session_name: d.high_session.clone(),
                });
            }

            // Check Low Session
            if !d.low_session.is_empty() && d.low_session != "Other" {
                daily_vec.push(ctx::SessionExtremeBaseContext {
                    trading_date: d.trading_date,
                    asset_id: d.asset_id.clone(),
                    extreme_type: "Low".to_string(),
                    extreme_session_name: d.low_session.clone(),
                });
            }

            daily_vec
        }).collect();

        if contexts.is_empty() { return Ok(Vec::new()); }

        let (counts, totals) = self.aggregate_ml_features(&contexts, intervals);

        // 2. Map to the ML Struct
        let results = counts.into_iter().map(|(key, interval_counts)| {
            let (asset_id, extreme_type, session_name) = key;
            let total_key = (asset_id.clone(), extreme_type.clone());
            let interval_totals = totals.get(&total_key);

            let get_stats = |name: &str| {
                let count = *interval_counts.get(name).unwrap_or(&0.0);
                let total = interval_totals.and_then(|t| t.get(name)).cloned().unwrap_or(0.0);
                let prob = if total > 0.0 { count / total } else { 0.0 };
                (prob, count, total)
            };

            let (p6, c6, t6) = get_stats("6M");
            let (p1, c1, t1) = get_stats("1Y");
            let (pa, ca, ta) = get_stats("ALL");

            mm::SessionExtremeBaseRateML {
                asset_id, session_name, extreme_type,
                p_6m: p6, count_6m: c6, total_6m: t6,
                p_1y: p1, count_1y: c1, total_1y: t1,
                p_all: pa, count_all: ca, total_all: ta,
            }
        }).collect();

        Ok(results)
    }

    pub async fn calculate_session_extreme_outcome_ml(
        &self,
        session_data: &[sm::SessionContextData],
        daily_extremes: &[sm::DailyContextData],
        intervals: &[sm::LookbackInterval],
    ) -> Result<Vec<mm::SessionExtremeOutcomeML>> {
        
        // 1. Prepare contexts (High and Low for every session record)
        // This helper joins the two datasets using the HashMap strategy we defined
        let contexts = ctx::prepare_session_extreme_contexts(session_data, daily_extremes);

        if contexts.is_empty() {
            return Ok(Vec::new());
        }

        // 2. Aggregate the features
        let (counts, totals) = self.aggregate_ml_features(&contexts, intervals);

        // 3. Map to the ML Wide Struct
        let results = counts.into_iter().map(|(key, interval_counts)| {
            let (asset_id, (ps2n, ps2b, ps1n, ps1b, extreme_type), extreme_session_name) = key;
            
            let total_key = (asset_id.clone(), (ps2n.clone(), ps2b.clone(), ps1n.clone(), ps1b.clone(), extreme_type.clone()));
            let interval_totals = totals.get(&total_key);

            let get_stats = |name: &str| {
                let count = *interval_counts.get(name).unwrap_or(&0.0);
                let total = interval_totals.and_then(|t| t.get(name)).cloned().unwrap_or(0.0);
                let prob = if total > 0.0 { count / total } else { 0.0 };
                (prob, count, total)
            };

            let (p_6m, c_6m, t_6m) = get_stats("6M");
            let (p_1y, c_1y, t_1y) = get_stats("1Y");
            let (p_all, c_all, t_all) = get_stats("ALL");

            mm::SessionExtremeOutcomeML {
                asset_id,
                ps2_name: ps2n, ps2_bias: ps2b,
                ps1_name: ps1n, ps1_bias: ps1b,
                extreme_type,
                extreme_session_name,
                p_6m, count_6m: c_6m, total_6m: t_6m,
                p_1y, count_1y: c_1y, total_1y: t_1y,
                p_all, count_all: c_all, total_all: t_all,
            }
        }).collect();

        Ok(results)
    }

    pub async fn calculate_session_extreme_lift_ml(
        &self,
        outcomes: Vec<mm::SessionExtremeOutcomeML>,
        base_rates: Vec<mm::SessionExtremeBaseRateML>,
    ) -> Result<Vec<mm::SessionExtremeLiftML>> {
        
        // 1. Create O(1) Lookup for Base Rates: (Asset, Session, Type) -> Rate
        let base_map: HashMap<(String, String, String), mm::SessionExtremeBaseRateML> = base_rates
            .into_iter()
            .map(|r| ((r.asset_id.clone(), r.session_name.clone(), r.extreme_type.clone()), r))
            .collect();

        let lifts = outcomes.into_iter().filter_map(|o| {
            // Find the base rate for this specific session forming this specific extreme
            let br = base_map.get(&(o.asset_id.clone(), o.extreme_session_name.clone(), o.extreme_type.clone()))?;

            let calc_lift = |pc: f64, pb: f64| (pc, pb, if pb > 0.0 { pc / pb } else { 1.0 });

            let (p6c, p6b, l6) = calc_lift(o.p_6m, br.p_6m);
            let (p1c, p1b, l1) = calc_lift(o.p_1y, br.p_1y);
            let (pac, pab, la) = calc_lift(o.p_all, br.p_all);

            Some(mm::SessionExtremeLiftML {
                asset_id: o.asset_id,
                ps2_name: o.ps2_name, ps2_bias: o.ps2_bias,
                ps1_name: o.ps1_name, ps1_bias: o.ps1_bias,
                extreme_type: o.extreme_type,
                extreme_session_name: o.extreme_session_name,
                p_cond_6m: p6c, p_base_6m: p6b, lift_6m: l6,
                p_cond_1y: p1c, p_base_1y: p1b, lift_1y: l1,
                p_cond_all: pac, p_base_all: pab, lift_all: la,
            })
        }).collect();

        Ok(lifts)
    }

    // Bar Extreme Events ML Calculations

    pub async fn calculate_bar_extreme_base_rates_ml(
        &self,
        daily_raw: &[sm::DailyContextData],
        intervals: &[sm::LookbackInterval],
    ) -> Result<Vec<mm::BarExtremeBaseRateML>> {
        let contexts: Vec<ctx::BarExtremeBaseContext> = daily_raw.iter().flat_map(|d| {
            vec![
                ctx::BarExtremeBaseContext {
                    trading_date: d.trading_date,
                    asset_id: d.asset_id.clone(),
                    extreme_type: "High".to_string(),
                    extreme_cb_num: d.high_bar,
                },
                ctx::BarExtremeBaseContext {
                    trading_date: d.trading_date,
                    asset_id: d.asset_id.clone(),
                    extreme_type: "Low".to_string(),
                    extreme_cb_num: d.low_bar,
                },
            ]
        }).collect();

        let (counts, totals) = self.aggregate_ml_features(&contexts, intervals);
        
        let results = counts.into_iter().map(|(key, interval_counts)| {
            let (asset_id, extreme_type, cb_num) = key;
            let total_key = (asset_id.clone(), extreme_type.clone());
            let stats = |name: &str| {
                let count = *interval_counts.get(name).unwrap_or(&0.0);
                let total = totals.get(&total_key).and_then(|t| t.get(name)).cloned().unwrap_or(0.0);
                (if total > 0.0 { count / total } else { 0.0 }, count, total)
            };
            let (p6, c6, t6) = stats("6M"); let (p1, c1, t1) = stats("1Y"); let (pa, ca, ta) = stats("ALL");
            mm::BarExtremeBaseRateML { asset_id, cb_num, extreme_type, p_6m: p6, count_6m: c6, total_6m: t6, p_1y: p1, count_1y: c1, total_1y: t1, p_all: pa, count_all: ca, total_all: ta }
        }).collect();
        Ok(results)
    }


    pub async fn calculate_bar_extreme_outcome_ml(
        &self,
        bar_data: &[sm::EightContextData],
        daily_extremes: &[sm::DailyContextData],
        intervals: &[sm::LookbackInterval],
    ) -> Result<Vec<mm::BarExtremeOutcomeML>> {
        
        // 1. Prepare contexts using the helper above
        let contexts = ctx::prepare_bar_extreme_contexts(bar_data, daily_extremes);
        if contexts.is_empty() { return Ok(Vec::new()); }

        // 2. Aggregate features
        let (counts, totals) = self.aggregate_ml_features(&contexts, intervals);

        // 3. Extract results using a custom mapper (since the key is a 5-tuple)
        let results = counts.into_iter().map(|(key, interval_counts)| {
            let (asset_id, (pb2n, pb2b, pb1n, pb1b, ext_type), ext_cb) = key;
            
            let total_key = (asset_id.clone(), (pb2n, pb2b.clone(), pb1n, pb1b.clone(), ext_type.clone()));
            let interval_totals = totals.get(&total_key);

            let get_stats = |name: &str| {
                let count = *interval_counts.get(name).unwrap_or(&0.0);
                let total = interval_totals.and_then(|t| t.get(name)).cloned().unwrap_or(0.0);
                (if total > 0.0 { count / total } else { 0.0 }, count, total)
            };

            let (p6, c6, t6) = get_stats("6M");
            let (p1, c1, t1) = get_stats("1Y");
            let (pa, ca, ta) = get_stats("ALL");

            mm::BarExtremeOutcomeML {
                asset_id, pb2_num: pb2n, pb2_bias: pb2b, pb1_num: pb1n, pb1_bias: pb1b,
                extreme_type: ext_type, extreme_cb_num: ext_cb,
                p_6m: p6, count_6m: c6, total_6m: t6,
                p_1y: p1, count_1y: c1, total_1y: t1,
                p_all: pa, count_all: ca, total_all: ta,
            }
        }).collect();

        Ok(results)
    }


    // --- BAR EXTREME LIFT ---
    pub async fn calculate_bar_extreme_lift_ml(
        &self,
        outcomes: Vec<mm::BarExtremeOutcomeML>,
        base_rates: Vec<mm::BarExtremeBaseRateML>,
    ) -> Result<Vec<mm::BarExtremeLiftML>> {
        let base_map: HashMap<(String, i32, String), mm::BarExtremeBaseRateML> = base_rates
            .into_iter().map(|r| ((r.asset_id.clone(), r.cb_num, r.extreme_type.clone()), r)).collect();

        let lifts = outcomes.into_iter().filter_map(|o| {
            let br = base_map.get(&(o.asset_id.clone(), o.extreme_cb_num, o.extreme_type.clone()))?;
            let calc = |pc: f64, pb: f64| (pc, pb, if pb > 0.0 { pc / pb } else { 1.0 });
            let (p6c, p6b, l6) = calc(o.p_6m, br.p_6m);
            let (p1c, p1b, l1) = calc(o.p_1y, br.p_1y);
            let (pac, pab, la) = calc(o.p_all, br.p_all);

            Some(mm::BarExtremeLiftML {
                asset_id: o.asset_id, pb2_num: o.pb2_num, pb2_bias: o.pb2_bias, pb1_num: o.pb1_num, pb1_bias: o.pb1_bias,
                extreme_type: o.extreme_type, extreme_cb_num: o.extreme_cb_num,
                p_cond_6m: p6c, p_base_6m: p6b, lift_6m: l6, p_cond_1y: p1c, p_base_1y: p1b, lift_1y: l1, p_cond_all: pac, p_base_all: pab, lift_all: la,
            })
        }).collect();
        Ok(lifts)
    }

    

}