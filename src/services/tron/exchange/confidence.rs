pub fn confidence_from_signals(
    seed_match: bool,
    sweeper_behavior: bool,
    withdrawal_pattern: bool,
) -> f32 {
    let mut score: f32 = 0.0;

    if seed_match {
        score += 0.7;
    }

    if sweeper_behavior {
        score += 0.2;
    }

    if withdrawal_pattern {
        score += 0.1;
    }
    score.min(1.0)
}
