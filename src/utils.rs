pub fn lerp(from: f64, to: f64, weight: f64) -> f64 {
    from + (to - from) * weight
}
