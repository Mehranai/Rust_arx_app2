use std::sync::Arc;

use clickhouse::Client;
use serde::Serialize;

use crate::services::tron::wallet_fingerprint::{
    WalletBehaviorSummary, WalletFingerprint, WalletFlowSummary, WalletIdentity,
    build_wallet_fingerprint,
};

const TRANSACTION_WEIGHT: f32 = 0.30;
const BEHAVIOR_WEIGHT: f32 = 0.20;
const TYPOLOGY_WEIGHT: f32 = 0.25;
const EXPOSURE_WEIGHT: f32 = 0.15;
const IDENTITY_WEIGHT: f32 = 0.10;

#[derive(Debug, Clone, Serialize)]
pub struct WalletAmlRiskAssessment {
    pub address: String,
    pub window_days: u16,
    pub risk_percent: u8,
    pub risk_score: f32,
    pub risk_level: String,
    pub confidence: f32,
    pub wallet_type: String,
    pub fingerprint_label: String,
    pub identity_type: String,
    pub components: Vec<AmlRiskComponent>,
    pub typologies: Vec<AmlTypology>,
    pub risk_factors: Vec<String>,
    pub protective_factors: Vec<String>,
    pub evidence: Vec<String>,
    pub source: AmlRiskSource,
}

#[derive(Debug, Clone, Serialize)]
pub struct AmlRiskComponent {
    pub name: String,
    pub score: f32,
    pub weight: f32,
    pub contribution: f32,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AmlTypology {
    pub typology: String,
    pub severity: String,
    pub confidence: f32,
    pub score: f32,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AmlRiskSource {
    pub model_version: String,
    pub fingerprint_generated_at_unix_ms: u64,
    pub sampled_event_limit: u64,
    pub is_truncated: bool,
}

pub async fn build_wallet_aml_risk_assessment(
    clickhouse: Arc<Client>,
    address: &str,
    window_days: Option<u16>,
    top_counterparties: Option<usize>,
    max_events: Option<u64>,
) -> anyhow::Result<WalletAmlRiskAssessment> {
    let fingerprint = build_wallet_fingerprint(
        clickhouse,
        address,
        window_days,
        top_counterparties,
        max_events,
    )
    .await?;

    Ok(assess_wallet_fingerprint(&fingerprint))
}

pub fn assess_wallet_fingerprint(fingerprint: &WalletFingerprint) -> WalletAmlRiskAssessment {
    let transaction = transaction_component(&fingerprint.flows);
    let behavior = behavior_component(&fingerprint.flows, &fingerprint.behavior);
    let typologies = detect_typologies(fingerprint);
    let typology = typology_component(&typologies);
    let exposure = exposure_component(fingerprint);
    let identity = identity_component(&fingerprint.identity, fingerprint);

    let mut components = vec![transaction, behavior, typology, exposure, identity];
    for component in &mut components {
        component.contribution = component.score * component.weight;
    }

    let mut risk_score = components
        .iter()
        .map(|component| component.contribution)
        .sum::<f32>();

    let mut protective_factors = protective_factors(fingerprint);
    if fingerprint.identity.identity_type == "exchange_service_wallet" && risk_score < 0.70 {
        risk_score *= 0.75;
        protective_factors.push("known_exchange_service_wallet_context".to_string());
    }

    if fingerprint.flows.total_transfers == 0 {
        risk_score = 0.0;
        protective_factors.push("no_observed_flow_history".to_string());
    }

    risk_score = clamp01(risk_score);
    let risk_percent = (risk_score * 100.0).round() as u8;
    let risk_level = risk_level(risk_percent).to_string();
    let confidence = confidence_score(fingerprint);
    let risk_factors = risk_factors(fingerprint, &components, &typologies);
    let evidence = assessment_evidence(fingerprint, &components, &typologies);

    WalletAmlRiskAssessment {
        address: fingerprint.address.clone(),
        window_days: fingerprint.window_days,
        risk_percent,
        risk_score,
        risk_level,
        confidence,
        wallet_type: fingerprint.wallet_type.clone(),
        fingerprint_label: fingerprint.fingerprint_label.clone(),
        identity_type: fingerprint.identity.identity_type.clone(),
        components,
        typologies,
        risk_factors,
        protective_factors,
        evidence,
        source: AmlRiskSource {
            model_version: "wallet_aml_risk_v1_rules".to_string(),
            fingerprint_generated_at_unix_ms: fingerprint.generated_at_unix_ms,
            sampled_event_limit: fingerprint.sampled_event_limit,
            is_truncated: fingerprint.is_truncated,
        },
    }
}

fn transaction_component(flows: &WalletFlowSummary) -> AmlRiskComponent {
    let avg_risk = flows.avg_tx_risk_score / 100.0;
    let max_risk = f32::from(flows.max_tx_risk_score) / 100.0;
    let high_risk_ratio = ratio(
        flows.high_risk_transfers as f32,
        flows.total_transfers as f32,
    );
    let score = clamp01(avg_risk * 0.45 + max_risk * 0.35 + high_risk_ratio * 0.20);

    let mut evidence = vec![
        format!("avg_tx_risk_score={:.1}", flows.avg_tx_risk_score),
        format!("max_tx_risk_score={}", flows.max_tx_risk_score),
    ];

    if flows.high_risk_transfers > 0 {
        evidence.push(format!(
            "high_risk_transfers={}/{}",
            flows.high_risk_transfers, flows.total_transfers
        ));
    }

    AmlRiskComponent {
        name: "transaction_risk".to_string(),
        score,
        weight: TRANSACTION_WEIGHT,
        contribution: 0.0,
        evidence,
    }
}

fn behavior_component(
    flows: &WalletFlowSummary,
    behavior: &WalletBehaviorSummary,
) -> AmlRiskComponent {
    let fan_in_score = if flows.unique_senders >= 25 && flows.unique_receivers <= 3 {
        0.35
    } else if flows.unique_senders >= 10 && flows.unique_receivers <= 3 {
        0.22
    } else {
        0.0
    };
    let fan_out_score = if flows.unique_receivers >= 25 && flows.unique_senders <= 3 {
        0.35
    } else if flows.unique_receivers >= 10 && flows.unique_senders <= 3 {
        0.22
    } else {
        0.0
    };
    let defi_score = (behavior.swap_ratio * 0.25) + (behavior.bridge_ratio * 0.35);
    let concentration_score =
        if behavior.counterparty_concentration >= 0.75 && flows.total_transfers >= 5 {
            0.18
        } else {
            0.0
        };
    let token_score = if behavior.token_diversity >= 8 {
        0.12
    } else if behavior.token_diversity >= 4 {
        0.06
    } else {
        0.0
    };

    let score = clamp01(
        fan_in_score
            + fan_out_score
            + defi_score
            + concentration_score
            + behavior.burst_score * 0.18
            + token_score,
    );

    AmlRiskComponent {
        name: "wallet_behavior".to_string(),
        score,
        weight: BEHAVIOR_WEIGHT,
        contribution: 0.0,
        evidence: vec![
            format!("unique_senders={}", flows.unique_senders),
            format!("unique_receivers={}", flows.unique_receivers),
            format!("burst_score={:.2}", behavior.burst_score),
            format!("swap_ratio={:.2}", behavior.swap_ratio),
            format!("bridge_ratio={:.2}", behavior.bridge_ratio),
            format!(
                "counterparty_concentration={:.2}",
                behavior.counterparty_concentration
            ),
            format!("token_diversity={}", behavior.token_diversity),
        ],
    }
}

fn typology_component(typologies: &[AmlTypology]) -> AmlRiskComponent {
    let score = clamp01(
        typologies
            .iter()
            .map(|typology| typology.score)
            .sum::<f32>(),
    );

    AmlRiskComponent {
        name: "typology_matches".to_string(),
        score,
        weight: TYPOLOGY_WEIGHT,
        contribution: 0.0,
        evidence: typologies
            .iter()
            .map(|typology| format!("{}:{:.2}", typology.typology, typology.confidence))
            .collect(),
    }
}

fn exposure_component(fingerprint: &WalletFingerprint) -> AmlRiskComponent {
    let max_sender_risk = fingerprint
        .senders
        .iter()
        .map(|counterparty| counterparty.max_risk_score)
        .max()
        .unwrap_or_default();
    let max_receiver_risk = fingerprint
        .receivers
        .iter()
        .map(|counterparty| counterparty.max_risk_score)
        .max()
        .unwrap_or_default();
    let risky_counterparty_share = fingerprint
        .senders
        .iter()
        .chain(fingerprint.receivers.iter())
        .filter(|counterparty| counterparty.max_risk_score >= 70)
        .map(|counterparty| counterparty.share_of_wallet_transfers)
        .sum::<f32>();

    let score = clamp01(
        (f32::from(max_sender_risk.max(max_receiver_risk)) / 100.0) * 0.55
            + risky_counterparty_share * 0.45,
    );

    AmlRiskComponent {
        name: "direct_counterparty_exposure".to_string(),
        score,
        weight: EXPOSURE_WEIGHT,
        contribution: 0.0,
        evidence: vec![
            format!("max_sender_risk={}", max_sender_risk),
            format!("max_receiver_risk={}", max_receiver_risk),
            format!("risky_counterparty_share={:.2}", risky_counterparty_share),
        ],
    }
}

fn identity_component(
    identity: &WalletIdentity,
    fingerprint: &WalletFingerprint,
) -> AmlRiskComponent {
    let identity_type = identity.identity_type.as_str();
    let score = if identity_type.contains("scam")
        || identity_type.contains("mixer")
        || identity_type.contains("sanction")
    {
        1.0
    } else if fingerprint.wallet_type == "exchange_deposit_funnel"
        || identity_type == "probable_exchange_deposit_wallet"
    {
        0.35
    } else if identity_type == "probable_sweeper_wallet"
        || identity_type == "probable_exchange_wallet"
    {
        0.25
    } else if identity_type == "exchange_service_wallet" {
        0.05
    } else {
        0.10
    };

    AmlRiskComponent {
        name: "identity_context".to_string(),
        score,
        weight: IDENTITY_WEIGHT,
        contribution: 0.0,
        evidence: vec![
            format!("identity_type={}", identity.identity_type),
            format!("identity_confidence={:.2}", identity.confidence),
            format!("identity_source={}", identity.source),
        ],
    }
}

fn detect_typologies(fingerprint: &WalletFingerprint) -> Vec<AmlTypology> {
    let flows = &fingerprint.flows;
    let behavior = &fingerprint.behavior;
    let mut typologies = Vec::new();

    if behavior.exchange_interaction_ratio >= 0.30
        && flows.unique_senders >= 10
        && flows.unique_receivers <= 5
        && flows.incoming_transfers > flows.outgoing_transfers
    {
        typologies.push(AmlTypology {
            typology: "exchange_cashout_funnel".to_string(),
            severity: "HIGH".to_string(),
            confidence: clamp01(0.55 + behavior.exchange_interaction_ratio * 0.40),
            score: 0.32,
            evidence: vec![
                format!(
                    "exchange_interaction_ratio={:.2}",
                    behavior.exchange_interaction_ratio
                ),
                format!("unique_senders={}", flows.unique_senders),
                format!("unique_receivers={}", flows.unique_receivers),
            ],
        });
    }

    if behavior.swap_ratio >= 0.30 && behavior.bridge_ratio >= 0.10 {
        typologies.push(AmlTypology {
            typology: "swap_bridge_layering".to_string(),
            severity: "HIGH".to_string(),
            confidence: clamp01(0.50 + behavior.swap_ratio * 0.25 + behavior.bridge_ratio * 0.35),
            score: 0.28,
            evidence: vec![
                format!("swap_ratio={:.2}", behavior.swap_ratio),
                format!("bridge_ratio={:.2}", behavior.bridge_ratio),
            ],
        });
    } else if behavior.bridge_ratio >= 0.25 {
        typologies.push(AmlTypology {
            typology: "bridge_heavy_layering".to_string(),
            severity: "MEDIUM".to_string(),
            confidence: clamp01(0.45 + behavior.bridge_ratio * 0.45),
            score: 0.18,
            evidence: vec![format!("bridge_ratio={:.2}", behavior.bridge_ratio)],
        });
    } else if behavior.swap_ratio >= 0.45 {
        typologies.push(AmlTypology {
            typology: "swap_heavy_obfuscation".to_string(),
            severity: "MEDIUM".to_string(),
            confidence: clamp01(0.45 + behavior.swap_ratio * 0.35),
            score: 0.16,
            evidence: vec![format!("swap_ratio={:.2}", behavior.swap_ratio)],
        });
    }

    if flows.unique_senders >= 25 && flows.unique_receivers <= 3 {
        typologies.push(AmlTypology {
            typology: "many_sources_to_few_destinations".to_string(),
            severity: "MEDIUM".to_string(),
            confidence: 0.72,
            score: 0.18,
            evidence: vec![
                format!("unique_senders={}", flows.unique_senders),
                format!("unique_receivers={}", flows.unique_receivers),
            ],
        });
    }

    if flows.unique_receivers >= 25 && flows.unique_senders <= 3 {
        typologies.push(AmlTypology {
            typology: "few_sources_to_many_destinations".to_string(),
            severity: "MEDIUM".to_string(),
            confidence: 0.72,
            score: 0.18,
            evidence: vec![
                format!("unique_senders={}", flows.unique_senders),
                format!("unique_receivers={}", flows.unique_receivers),
            ],
        });
    }

    if behavior.burst_score >= 0.75 && flows.total_transfers >= 10 {
        typologies.push(AmlTypology {
            typology: "bursty_flow_activity".to_string(),
            severity: "MEDIUM".to_string(),
            confidence: clamp01(behavior.burst_score),
            score: 0.12,
            evidence: vec![format!("burst_score={:.2}", behavior.burst_score)],
        });
    }

    typologies
}

fn risk_factors(
    fingerprint: &WalletFingerprint,
    components: &[AmlRiskComponent],
    typologies: &[AmlTypology],
) -> Vec<String> {
    let mut factors = fingerprint.risk_flags.clone();

    for typology in typologies {
        factors.push(format!("typology:{}", typology.typology));
    }

    for component in components
        .iter()
        .filter(|component| component.score >= 0.60)
    {
        factors.push(format!("high_component:{}", component.name));
    }

    factors.sort();
    factors.dedup();
    factors
}

fn protective_factors(fingerprint: &WalletFingerprint) -> Vec<String> {
    let mut factors = Vec::new();

    if fingerprint.identity.identity_type == "exchange_service_wallet" {
        factors.push("attributed_exchange_service_wallet".to_string());
    }

    if fingerprint.flows.max_tx_risk_score < 25 && fingerprint.flows.total_transfers >= 3 {
        factors.push("no_high_risk_transactions_observed".to_string());
    }

    if fingerprint.behavior.bridge_ratio == 0.0 && fingerprint.behavior.swap_ratio < 0.10 {
        factors.push("low_obfuscation_behavior".to_string());
    }

    factors
}

fn assessment_evidence(
    fingerprint: &WalletFingerprint,
    components: &[AmlRiskComponent],
    typologies: &[AmlTypology],
) -> Vec<String> {
    let mut evidence = vec![
        format!("fingerprint_label={}", fingerprint.fingerprint_label),
        format!("wallet_type={}", fingerprint.wallet_type),
        format!("identity_type={}", fingerprint.identity.identity_type),
        format!("total_transfers={}", fingerprint.flows.total_transfers),
        format!(
            "unique_transactions={}",
            fingerprint.flows.unique_transactions
        ),
    ];

    for item in &fingerprint.evidence {
        evidence.push(format!("fingerprint:{}", item));
    }

    for typology in typologies {
        evidence.push(format!(
            "typology:{} severity={} confidence={:.2}",
            typology.typology, typology.severity, typology.confidence
        ));
    }

    for component in components {
        evidence.push(format!(
            "component:{} score={:.2} weight={:.2}",
            component.name, component.score, component.weight
        ));
    }

    evidence
}

fn confidence_score(fingerprint: &WalletFingerprint) -> f32 {
    let volume_confidence = if fingerprint.flows.total_transfers >= 200 {
        0.90
    } else if fingerprint.flows.total_transfers >= 50 {
        0.80
    } else if fingerprint.flows.total_transfers >= 10 {
        0.65
    } else if fingerprint.flows.total_transfers >= 3 {
        0.45
    } else {
        0.25
    };

    let truncation_penalty = if fingerprint.is_truncated { 0.08 } else { 0.0 };

    clamp01(fingerprint.confidence * 0.60 + volume_confidence * 0.40 - truncation_penalty)
}

fn risk_level(risk_percent: u8) -> &'static str {
    match risk_percent {
        0..=24 => "LOW",
        25..=59 => "MEDIUM",
        60..=84 => "HIGH",
        _ => "CRITICAL",
    }
}

fn ratio(numerator: f32, denominator: f32) -> f32 {
    if denominator <= 0.0 {
        0.0
    } else {
        numerator / denominator
    }
}

fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::tron::wallet_fingerprint::{
        WalletBehaviorSummary, WalletCounterpartyFingerprint, WalletFlowSummary, WalletIdentity,
    };

    fn identity(identity_type: &str) -> WalletIdentity {
        WalletIdentity {
            address: "TWallet".to_string(),
            identity_type: identity_type.to_string(),
            entity_id: None,
            entity_name: None,
            entity_type: None,
            exchange_name: None,
            exchange_role: None,
            confidence: 0.80,
            source: "test".to_string(),
            tags: Vec::new(),
        }
    }

    fn fingerprint(
        identity: WalletIdentity,
        flows: WalletFlowSummary,
        behavior: WalletBehaviorSummary,
    ) -> WalletFingerprint {
        WalletFingerprint {
            address: "TWallet".to_string(),
            window_days: 90,
            sampled_event_limit: 20_000,
            is_truncated: false,
            fingerprint_label: "Test wallet".to_string(),
            wallet_type: "wallet".to_string(),
            confidence: 0.75,
            risk_score: 0.0,
            identity,
            flows,
            behavior,
            dominant_tokens: Vec::new(),
            senders: Vec::new(),
            receivers: Vec::new(),
            risk_flags: Vec::new(),
            evidence: Vec::new(),
            generated_at_unix_ms: 1,
        }
    }

    fn base_flows() -> WalletFlowSummary {
        WalletFlowSummary {
            total_transfers: 40,
            unique_transactions: 35,
            incoming_transfers: 30,
            outgoing_transfers: 10,
            unique_senders: 30,
            unique_receivers: 2,
            total_volume_in_raw: "1000".to_string(),
            total_volume_out_raw: "900".to_string(),
            avg_tx_risk_score: 35.0,
            max_tx_risk_score: 85,
            high_risk_transfers: 4,
        }
    }

    fn base_behavior() -> WalletBehaviorSummary {
        WalletBehaviorSummary {
            first_seen_timestamp: Some(0),
            last_seen_timestamp: Some(86_400_000),
            observed_days: 1.0,
            active_days: 1,
            active_hours: vec![1, 2],
            avg_tx_interval_seconds: 60.0,
            burst_score: 0.80,
            inbound_outbound_ratio: 3.0,
            counterparty_concentration: 0.30,
            token_diversity: 4,
            contract_call_ratio: 0.40,
            swap_ratio: 0.35,
            bridge_ratio: 0.20,
            exchange_interaction_ratio: 0.55,
        }
    }

    #[test]
    fn high_risk_cashout_pattern_scores_high() {
        let mut fingerprint = fingerprint(identity("wallet"), base_flows(), base_behavior());
        fingerprint.senders.push(WalletCounterpartyFingerprint {
            address: "TRiskySource".to_string(),
            direction: "sender".to_string(),
            relationship_label: "direct high-risk source".to_string(),
            identity: identity("wallet"),
            transfer_count: 6,
            unique_transactions: 6,
            total_volume_raw: "250".to_string(),
            first_seen_timestamp: 0,
            last_seen_timestamp: 1,
            tokens: vec!["USDT".to_string()],
            dominant_token: Some("USDT".to_string()),
            avg_risk_score: 82.0,
            max_risk_score: 92,
            share_of_wallet_transfers: 0.20,
        });

        let assessment = assess_wallet_fingerprint(&fingerprint);

        assert!(assessment.risk_percent >= 60);
        assert_eq!(assessment.risk_level, "HIGH");
        assert!(
            assessment
                .typologies
                .iter()
                .any(|typology| typology.typology == "exchange_cashout_funnel")
        );
        assert!(
            assessment
                .typologies
                .iter()
                .any(|typology| typology.typology == "swap_bridge_layering")
        );
    }

    #[test]
    fn attributed_exchange_service_wallet_gets_context_discount() {
        let mut flows = base_flows();
        flows.avg_tx_risk_score = 5.0;
        flows.max_tx_risk_score = 10;
        flows.high_risk_transfers = 0;
        flows.unique_senders = 4;
        flows.unique_receivers = 4;
        flows.incoming_transfers = 5;
        flows.outgoing_transfers = 5;

        let mut behavior = base_behavior();
        behavior.swap_ratio = 0.0;
        behavior.bridge_ratio = 0.0;
        behavior.exchange_interaction_ratio = 0.0;
        behavior.burst_score = 0.0;

        let assessment = assess_wallet_fingerprint(&fingerprint(
            identity("exchange_service_wallet"),
            flows,
            behavior,
        ));

        assert!(assessment.risk_percent <= 24);
        assert_eq!(assessment.risk_level, "LOW");
        assert!(
            assessment
                .protective_factors
                .iter()
                .any(|factor| factor == "known_exchange_service_wallet_context")
        );
    }

    #[test]
    fn direct_high_risk_counterparty_increases_exposure_component() {
        let mut fingerprint = fingerprint(identity("wallet"), base_flows(), base_behavior());
        fingerprint.senders.push(WalletCounterpartyFingerprint {
            address: "TRisky".to_string(),
            direction: "sender".to_string(),
            relationship_label: "direct sender wallet".to_string(),
            identity: identity("wallet"),
            transfer_count: 4,
            unique_transactions: 4,
            total_volume_raw: "100".to_string(),
            first_seen_timestamp: 0,
            last_seen_timestamp: 1,
            tokens: vec!["TRX".to_string()],
            dominant_token: Some("TRX".to_string()),
            avg_risk_score: 80.0,
            max_risk_score: 90,
            share_of_wallet_transfers: 0.25,
        });

        let assessment = assess_wallet_fingerprint(&fingerprint);
        let exposure = assessment
            .components
            .iter()
            .find(|component| component.name == "direct_counterparty_exposure")
            .expect("exposure component");

        assert!(exposure.score >= 0.60);
    }
}
