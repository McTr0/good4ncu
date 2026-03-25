/// Convert yuan (float, e.g., 99.99) to cents (integer, e.g., 9999).
/// Rounds to the nearest cent.
pub fn yuan_to_cents(yuan: f64) -> i64 {
    (yuan * 100.0).round() as i64
}

/// Convert cents (integer, e.g., 9999) to yuan (float, e.g., 99.99).
pub fn cents_to_yuan(cents: i64) -> f64 {
    cents as f64 / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yuan_to_cents() {
        assert_eq!(yuan_to_cents(99.99), 9999);
        assert_eq!(yuan_to_cents(0.1), 10);
        assert_eq!(yuan_to_cents(100.0), 10000);
    }

    #[test]
    fn test_cents_to_yuan() {
        assert!((cents_to_yuan(9999) - 99.99).abs() < f64::EPSILON);
        assert!((cents_to_yuan(10) - 0.1).abs() < f64::EPSILON);
        assert!((cents_to_yuan(10000) - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_round_trip() {
        let original = 49.99_f64;
        let cents = yuan_to_cents(original);
        let back = cents_to_yuan(cents);
        assert!((back - original).abs() < f64::EPSILON);
    }

    #[test]
    fn test_yuan_to_cents_rounding() {
        // Half-up rounding
        assert_eq!(yuan_to_cents(0.005), 1);
        assert_eq!(yuan_to_cents(0.004), 0);
        assert_eq!(yuan_to_cents(0.015), 2);
    }

    #[test]
    fn test_zero_values() {
        assert_eq!(yuan_to_cents(0.0), 0);
        assert_eq!(cents_to_yuan(0), 0.0);
    }

    #[test]
    fn test_large_values() {
        assert_eq!(yuan_to_cents(1_000_000.0), 100_000_000);
        assert_eq!(cents_to_yuan(100_000_000), 1_000_000.0);
    }

    #[test]
    fn test_cents_to_yuan_precision() {
        // Ensure no floating point drift for common prices
        assert!((cents_to_yuan(4999) - 49.99).abs() < f64::EPSILON);
        assert!((cents_to_yuan(199) - 1.99).abs() < f64::EPSILON);
    }
}
