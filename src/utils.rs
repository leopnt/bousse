pub fn lerp(from: f64, to: f64, weight: f64) -> f64 {
    from + (to - from) * weight
}

pub fn to_min_sec_millis_str(time_sec: f64) -> String {
    let integer_part = time_sec.trunc() as u64;
    let fractional_part = time_sec.fract();

    let minutes = (integer_part % 3600) / 60;
    let seconds = integer_part % 60;
    let millis = (fractional_part * 1000.0).round();

    if time_sec >= 60.0 * 60.0  {
        return "INF".to_string();
    }

    format!("{:02}:{:02}:{:03}", minutes, seconds, millis)
}
