# 28-Nodes-Quant-System (28NQS)  --Daily Composite Score (DCS) Framework Documentation

## 1. Introduction: A Data-Driven Conditional Scoring System

The **Daily Composite Score (DCS) Framework** is a robust, two-stage data pipeline designed to generate a daily, signed directional conviction score for an asset. The system leverages **conditional probability analysis** across multiple market dimensions to filter out noise and isolate statistical edge.

The framework operates on the principle of **$P(\text{Outcome} \mid \text{Current Context})$**, where the 'Current Context' is defined by the preceding day's bias ($\mathbf{P}_{\text{Day Type}}$) and the day-of-week transition ($\mathbf{DOW}_{\text{Transition}}$).

### System Architecture

Loading ..................
