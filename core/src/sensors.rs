//! Sensor logic ported from the original's monitor and pages.

/// The original's CPU temperature smoothing: the second reading after a cold
/// start is taken at 80%; rises move 10% of the way, falls 90% (slow up, fast down).
#[derive(Clone, Copy, Debug, Default)]
pub struct TemperatureSmoother {
    last: Option<f64>,
}

impl TemperatureSmoother {
    pub fn push(&mut self, value: f64) -> f64 {
        let next = match self.last {
            None => value,
            Some(last) if last < 10.0 => value * 4.0 / 5.0,
            Some(last) if value > last => (last * 9.0 + value) / 10.0,
            Some(last) => (last + value * 9.0) / 10.0,
        };
        self.last = Some(next);
        next
    }

    pub fn reset(&mut self) {
        self.last = None;
    }
}

/// Temperature colour state (legend: green low to moderate, orange high, red critical).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TempState {
    Unknown,
    Normal,
    Warning,
    Overheating,
}

pub fn temp_state(celsius: Option<f64>) -> TempState {
    match celsius {
        None => TempState::Unknown,
        Some(t) if t <= 0.0 => TempState::Unknown,
        Some(t) if t >= 95.0 => TempState::Overheating,
        Some(t) if t >= 85.0 => TempState::Warning,
        Some(_) => TempState::Normal,
    }
}

pub fn c_to_f(celsius: f64) -> f64 {
    celsius * 9.0 / 5.0 + 32.0
}

/// "58°C" / "136°F" / "--", as the original formats temperatures.
pub fn format_temp(celsius: Option<f64>, fahrenheit: bool) -> String {
    match celsius {
        Some(c) if c > 0.0 => {
            if fahrenheit {
                format!("{}°F", c_to_f(c).round() as i64)
            } else {
                format!("{}°C", c.round() as i64)
            }
        }
        _ => "--".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoothing_rises_slowly_and_falls_fast() {
        let mut s = TemperatureSmoother::default();
        assert_eq!(s.push(60.0), 60.0);
        assert!((s.push(70.0) - 61.0).abs() < 1e-9);
        assert!((s.push(50.0) - 51.1).abs() < 1e-9);
        let mut cold = TemperatureSmoother::default();
        cold.push(5.0);
        assert!((cold.push(50.0) - 40.0).abs() < 1e-9);
    }

    #[test]
    fn temperatures_format_like_the_original() {
        assert_eq!(format_temp(Some(49.4), false), "49°C");
        assert_eq!(format_temp(Some(49.4), true), "121°F");
        assert_eq!(format_temp(None, false), "--");
        assert_eq!(temp_state(Some(50.0)), TempState::Normal);
        assert_eq!(temp_state(Some(90.0)), TempState::Warning);
        assert_eq!(temp_state(Some(97.0)), TempState::Overheating);
        assert_eq!(temp_state(None), TempState::Unknown);
    }
}
