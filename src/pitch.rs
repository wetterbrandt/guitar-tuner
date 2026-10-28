#[derive(Debug, Clone, Copy)]
pub struct Pitch {
    pub frequency: f32,
    pub midi: i32,
    pub cents: f32,
}

impl Pitch {
    pub fn note_name(&self) -> &'static str {
        const NAMES: [&str; 12] = [
            "C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B",
        ];
        NAMES[self.midi.rem_euclid(12) as usize]
    }
    pub fn octave(&self) -> i32 {
        self.midi / 12 - 1
    }
}

/// YIN pitch detection, constrained to the useful 70–1,200 Hz guitar range.
pub fn detect(samples: &[f32], sample_rate: u32) -> Option<Pitch> {
    if samples.len() < 2048 {
        return None;
    }
    let rms = (samples.iter().map(|x| x * x).sum::<f32>() / samples.len() as f32).sqrt();
    if rms < 0.003 {
        return None;
    }
    let mean = samples.iter().sum::<f32>() / samples.len() as f32;
    let signal: Vec<f32> = samples.iter().map(|x| x - mean).collect();
    let min_tau = (sample_rate as f32 / 1200.0) as usize;
    let max_tau = ((sample_rate as f32 / 70.0) as usize).min(signal.len() / 2 - 1);
    let mut difference = vec![0.0; max_tau + 1];
    for tau in 1..=max_tau {
        difference[tau] = signal[..signal.len() - tau]
            .iter()
            .zip(&signal[tau..])
            .map(|(a, b)| {
                let d = a - b;
                d * d
            })
            .sum();
    }
    let mut running = 0.0;
    for tau in 1..=max_tau {
        running += difference[tau];
        difference[tau] = if running == 0.0 {
            1.0
        } else {
            difference[tau] * tau as f32 / running
        };
    }
    let mut tau = min_tau;
    while tau < max_tau {
        if difference[tau] < 0.12 {
            while tau + 1 <= max_tau && difference[tau + 1] < difference[tau] {
                tau += 1;
            }
            break;
        }
        tau += 1;
    }
    if tau >= max_tau || difference[tau] > 0.3 {
        return None;
    }
    let better_tau = if tau > 1 && tau < max_tau {
        let (left, center, right) = (difference[tau - 1], difference[tau], difference[tau + 1]);
        let denominator = 2.0 * center - left - right;
        if denominator.abs() > 1e-9 {
            tau as f32 + (right - left) / (2.0 * denominator)
        } else {
            tau as f32
        }
    } else {
        tau as f32
    };
    let frequency = sample_rate as f32 / better_tau;
    let midi_float = 69.0 + 12.0 * (frequency / 440.0).log2();
    let midi = midi_float.round() as i32;
    Some(Pitch {
        frequency,
        midi,
        cents: (midi_float - midi as f32) * 100.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_low_e() {
        let rate = 48_000;
        let samples: Vec<f32> = (0..8192)
            .map(|i| (i as f32 * std::f32::consts::TAU * 82.4069 / rate as f32).sin() * 0.5)
            .collect();
        let pitch = detect(&samples, rate).unwrap();
        assert_eq!((pitch.note_name(), pitch.octave()), ("E", 2));
        assert!(pitch.cents.abs() < 0.1);
    }
}
