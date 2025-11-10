# 28-Nodes-Quant-System (28NQS)  --Daily Composite Score (DCS) Framework Documentation

## 1. Introduction: A Data-Driven Conditional Scoring System

The **Daily Composite Score (DCS) Framework** is a robust, two-stage data pipeline designed to generate a daily, signed directional conviction score for an asset. The system leverages **conditional probability analysis** across multiple market dimensions to filter out noise and isolate statistical edge.

The framework operates on the principle of **$P(\text{Outcome} \mid \text{Current Context})$**, where the 'Current Context' is defined by the preceding day's bias ($\mathbf{P}_{\text{Day Type}}$) and the day-of-week transition ($\mathbf{DOW}_{\text{Transition}}$).

### System Architecture

The DCS is the product of two primary pipelines:

1. **Heavy Lifting Pipeline (Metric Aggregation):** Calculates and centralizes all $\mathbf{M_n}$ factor probabilities and $\mathbf{DBS}$ scores.
    
2. **System Logic Pipeline (DCS Calculation):** Combines the strongest **Base Directional Signal ($\mathbf{DBS}$)** with three multiplicative, non-linear **Confidence/Risk Factors ($\mathbf{F}_{\text{n}}$)** to produce the final, signed $\mathbf{DCS}$.
    

The final $\mathbf{DCS}$ is expressed as a percentage score (e.g., $+65.0\%$) and is subject to the immediate, non-linear dampening effect of the $\mathbf{F}_{\text{Reversal}}$ factor.

---

## 2. Conditional Metric Factors ($\mathbf{M_n}$)

The core of the system relies on five conditional metrics ($\mathbf{M_1}$ through $\mathbf{M_5}$) that assess specific aspects of market behavior. These metrics are aggregated based on the asset's history under the same $\mathbf{P}_{\text{Day Type}}$ ('Bullish', 'Bearish', or 'Consolidation') and $\mathbf{DOW}_{\text{Transition}}$ ('Mon $\rightarrow$ Tue', 'Wed $\rightarrow$ Thu', etc.).

### **$\mathbf{D.M2}$: Daily Base Score (DBS) - The Base Signal**

#### Introduction

$\mathbf{D.M2}$ is the primary structural conviction score. It measures the statistical difference in the probability of breaking the **Prior Week High ($\mathbf{PWH}$)** versus breaking the **Prior Week Low ($\mathbf{PWL}$)**, conditioned on the current context. This score is calculated across three look-back periods (Short-Term, Long-Term, and Year-to-Date) to account for time-dependent regime shifts.

#### Mathematical Formula

The $\mathbf{DBS}$ is defined as the signed difference between the conditional $\mathbf{PWH}$ break probability and the conditional $\mathbf{PWL}$ break probability.

$$\mathbf{DBS}_{\text{Timeframe}} = P(\mathbf{PWH}_{\text{Break}} \mid \text{Context}) - P(\mathbf{PWL}_{\text{Break}} \mid \text{Context})$$

Where $P(\mathbf{PWH}_{\text{Break}} \mid \text{Context})$ is calculated as:

$$P(\mathbf{PWH}_{\text{Break}} \mid \text{Context}) = \frac{N_{\mathbf{PWH}_{\text{Break}}}}{D_{\text{Context}}}$$

|**Variable**|**Description**|
|---|---|
|$\mathbf{Context}$|Defined by $(\mathbf{P}_{\text{Day Type}}, \mathbf{DOW}_{\text{Transition}})$.|
|$N_{\mathbf{PWH}_{\text{Break}}}$|Count of historical days in $\mathbf{Context}$ where $High > \mathbf{PWH}$.|
|$N_{\mathbf{PWL}_{\text{Break}}}$|Count of historical days in $\mathbf{Context}$ where $Low < \mathbf{PWL}$.|
|$D_{\text{Context}}$|Total count of historical days in $\mathbf{Context}$ within the $\mathbf{Timeframe}$.|

The system produces three scores: $\mathbf{DBS}_{\text{ST}}$ (Short-Term: 6 months), $\mathbf{DBS}_{\text{LT}}$ (Long-Term: 3 years), and $\mathbf{DBS}_{\text{YTD}}$ (Year-to-Date).

#### Example: $\mathbf{DBS}_{\text{ST}}$ Calculation

Assume the $\mathbf{Context}$ is **('Bullish', 'Wed $\rightarrow$ Thu')** over the last 6 months, with $D_{\text{Context}} = 48$ events.

|**Event**|**Count (Nn​)**|**Probability (Pn​)**|
|---|---|---|
|$\mathbf{PWH}$ Break|$N_{\mathbf{PWH}} = 38$|$38 / 48 = 0.7917$|
|$\mathbf{PWL}$ Break|$N_{\mathbf{PWL}} = 5$|$5 / 48 = 0.1042$|
|**Result**||$\mathbf{DBS}_{\text{ST}} = 0.7917 - 0.1042 = \mathbf{+0.6875}$|

### **$\mathbf{D.M3}$: Continuation Probability - $\mathbf{F}_{\text{Commitment}}$ Basis**

#### Introduction

$\mathbf{D.M3}$ measures the probability of a directional continuation, conditioned on the previous day's bias. It assesses the market's **commitment** to an established daily trend.

#### Mathematical Formula

The Continuation Probability $P_{\text{Cont}}$ is:

$$P_{\text{Cont}} = P(\mathbf{C}_{\text{Bias}} = \mathbf{P}_{\text{Bias}} \mid \mathbf{P}_{\text{Day Type}}, \mathbf{DOW}_{\text{Transition}})$$

$$P_{\text{Cont}} = \frac{N_{\text{Continuation}}}{D_{\text{Context}}}$$

|**Variable**|**Description**|
|---|---|
|$N_{\text{Continuation}}$|Count of historical instances where the current day's bias ($\mathbf{C}_{\text{Bias}}$) matched the previous day's bias ($\mathbf{P}_{\text{Bias}}$).|
|$D_{\text{Context}}$|Total historical count for the given $\mathbf{Context}$.|

This probability is then translated into the $\mathbf{F}_{\text{Commitment}}$ factor (see Section 3).

### **$\mathbf{D.M4}$: Follow-Through (FT) Probability - $\mathbf{F}_{\text{Sustainability}}$ Basis**

#### Introduction

$\mathbf{D.M4}$ measures the likelihood that a prior structural break (e.g., **Prior Day High/Low**) results in a follow-through (FT) closure. This metric assesses the **sustainability** of momentum once a directional threshold is cleared.

#### Mathematical Formula

$P_{\text{FT}}$ is the conditional probability that a day closes in the direction of the prior day's break:

$$P_{\text{FT}} = P(\mathbf{C}_{\text{Close}} = \text{Dir} \mid \mathbf{P}_{\text{Break}} = \text{Dir})$$

$$P_{\text{FT}_{\text{Dir}}} = \frac{N_{\text{Break } \rightarrow \text{ FT Closure}}}{N_{\text{Prior Break}}}$$

|**Variable**|**Description**|
|---|---|
|$N_{\text{Break } \rightarrow \text{ FT Closure}}$|Count of historical breaks (e.g., Bullish Break) that resulted in a directional closing candle (e.g., Bullish Close).|
|$N_{\text{Prior Break}}$|Total count of historical breaks in the target direction (e.g., $High > \mathbf{PDH}$).|

This probability is translated into the $\mathbf{F}_{\text{Sustainability}}$ factor (see Section 3).

### **$\mathbf{D.M5}$: Reversal Risk - $\mathbf{F}_{\text{Reversal}}$ Basis**

#### Introduction

$\mathbf{D.M5}$ measures the inherent risk of a directional reversal _on the day of a break_. Specifically, it calculates the probability that a break above $\mathbf{PDH}$ resulted in a close near the low, or a break below $\mathbf{PDL}$ resulted in a close near the high (a 'wick-out' reversal). This serves as the primary **risk filter**.

#### Mathematical Formula

$\mathbf{D.M5}$ is the probability of a high-wick reversal following a directional break:

$$P_{\text{Rev}} = P(\mathbf{C}_{\text{Wick}} \mid \mathbf{P}_{\text{Break}})$$

$$\mathbf{D.M5}_{\text{Bullish}} = \frac{N_{\text{Bullish Break AND Reversal Wick }}}{N_{\text{Bullish Break}}}$$

Where a Reversal Wick is defined by a close near the extreme opposite of the break direction:

$$\mathbf{Wick}_{\text{Bullish}} \equiv \frac{High - Close}{High - Low} \geq 0.70$$

$$\mathbf{Wick}_{\text{Bearish}} \equiv \frac{Close - Low}{High - Low} \leq 0.30$$

This raw probability is used to calculate the non-linear $\mathbf{F}_{\text{Reversal}}$ dampening factor.

---

## 3. The Daily Composite Score (DCS) System

The $\mathbf{DCS}$ system integrates the directional score ($\mathbf{DBS}$) with three multiplicative factors ($\mathbf{F}_{\text{n}}$) derived from the $\mathbf{M_n}$ probabilities.

### 3.1. Base Signal Selection ($\mathbf{DBS}_{\text{Strongest}}$)

The system selects the most convicted directional score from the three $\mathbf{DBS}$ timeframes ($\mathbf{ST}$, $\mathbf{LT}$, $\mathbf{YTD}$) provided the absolute magnitude exceeds a threshold of $0.25$.

$$\mathbf{DBS}_{\text{Strongest}} = \max_{\text{Timeframe} \in \{ST, LT, YTD\}} \left( \text{Signed Score}_{\text{Timeframe}} \right) \quad \text{if } |\mathbf{DBS}| > 0.25$$

If no score exceeds $0.25$, $\mathbf{DBS}_{\text{Strongest}}$ is set to $0.0$.

### 3.2. Factor Translation ($\mathbf{F}_{\text{n}}$)

The $\mathbf{M_n}$ probabilities are translated into non-linear multiplicative factors $\mathbf{F}_{\text{n}}$ to ensure they reward high confidence metrics disproportionately.

|**Factor**|**Metric Origin**|**Mathematical Formula**|**Interpretation**|
|---|---|---|---|
|$\mathbf{F}_{\text{Reversal}}$|$\mathbf{D.M5}$ (Raw Probability $P_{\text{Rev}}$)|$\mathbf{F}_{\text{Reversal}} = 1.0 - 0.2 \times P_{\text{Rev}}$|**Risk Filter:** Dampens $\mathbf{DCS}$ by up to $20\%$ based on reversal risk.|
|$\mathbf{F}_{\text{DM1}}$|$\mathbf{D.M1}$ (Prior Weekly Type)|$\mathbf{F}_{\text{DM1}} = \begin{cases} 1.05 & \text{if } \text{Alignment} \\ 0.95 & \text{if } \text{Misalignment} \\ 1.00 & \text{else} \end{cases}$|**Structural Confirmation:** Rewards alignment between $\mathbf{P}_{\text{Weekly Type}}$ and $\mathbf{DBS}_{\text{ST}}$ direction.|
|$\mathbf{F}_{\text{Commitment}}$|$\mathbf{D.M3}$ (Continuation $P_{\text{Cont}}$)|$\mathbf{F}_{\text{Commitment}} = 0.8 + 0.4 \times P_{\text{Cont}}$|**Momentum:** Rewards high continuation probability (factor range $0.8 \text{ to } 1.2$).|
|$\mathbf{F}_{\text{Sustainability}}$|$\mathbf{D.M4}$ (Follow-Through $P_{\text{FT}}$)|$\mathbf{F}_{\text{Sustainability}} = \frac{P_{\text{FT}} + 0.1}{1.0 - P_{\text{FT}} + 0.1}$|**Efficacy:** Rewards high follow-through probability (non-linear boost).|

### 3.3. Final DCS Formula

The $\mathbf{DCS}$ is calculated by multiplying the strongest $\mathbf{DBS}$ signal by the two primary filtering factors ($\mathbf{F}_{\text{Reversal}}$, $\mathbf{F}_{\text{DM1}}$). The $\mathbf{F}_{\text{Commitment}}$ and $\mathbf{F}_{\text{Sustainability}}$ are logged for analysis but are **not** currently included in the final $\mathbf{DCS}$ multiplication, maintaining a lean risk-filtered score.

$$\mathbf{DCS} = \mathbf{DBS}_{\text{Strongest}} \times \mathbf{F}_{\text{Reversal}} \times \mathbf{F}_{\text{DM1}}$$

### 3.4. Example: $\mathbf{DCS}$ Calculation

Assume the system runs a calculation for **assets:US2000** for a given day:

|**Metric**|**Value**|**Interpretation**|
|---|---|---|
|$\mathbf{DBS}_{\text{Strongest}}$|$\mathbf{+0.6875}$|Base Conviction is **Long** (from $\mathbf{DBS}_{\text{ST}}$).|
|$\mathbf{P}_{\text{Weekly Type}}$|**Bullish**|Aligned with the $\mathbf{DBS}_{\text{Strongest}}$ direction.|
|$\mathbf{D.M5}_{\text{Bearish Rev. Risk}}$|$\mathbf{0.15}$ (or 15%)|Low risk of an inverse wick-out reversal.|

Step 1: Calculate $\mathbf{F}_{\text{Reversal}}$

$$P_{\text{Rev}} = 0.15$$

$$\mathbf{F}_{\text{Reversal}} = 1.0 - 0.2 \times 0.15 = 1.0 - 0.03 = \mathbf{0.97}$$

Step 2: Calculate $\mathbf{F}_{\text{DM1}}$

Since the $\mathbf{DBS}$ is Bullish/Positive and the $\mathbf{P}_{\text{Weekly Type}}$ is Bullish, they are aligned.

$$\mathbf{F}_{\text{DM1}} = \mathbf{1.05}$$

Step 3: Calculate Final $\mathbf{DCS}$

$$\mathbf{DCS} = (+0.6875) \times 0.97 \times 1.05 = \mathbf{+0.7011}$$

Final Score and Classification:

$$\mathbf{DCS}_{\text{Final}} = +0.7011 \times 100 = \mathbf{+70.11\%}$$

- **Classification:** Since $\mathbf{70.11\%} > 50\%$, the signal is **High Conviction Long**.
    
- **Logging:** The raw $\mathbf{D.M5}$ risk of $15\%$ is logged in the `F_Reversal` column space.