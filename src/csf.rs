#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Csf {
    pub a: f32,
    pub ω: f32,
    pub σ: f32,
    pub k: f32,
}

impl Csf {
    pub fn apply(&self, f: f32) -> f32 {
        self.a * ((-f / self.ω).exp() - self.k * (-(f / self.σ).powi(2)).exp())
    }

    pub fn plot_points(
        &self,
        min: f32,
        max: f32,
        points: usize,
    ) -> impl Iterator<Item = (f32, f32)> {
        let self_clone = self.clone();
        (0..points).map(move |i| {
            let norm = i as f32 / (points - 1) as f32;
            let adjusted = norm * (max - min) + min;
            (adjusted, self_clone.apply(adjusted))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eq() {
        let csf = Csf {
            a: 90.0,
            ω: 32.0,
            σ: 93.0,
            k: 10000000000.0,
        };
        assert_eq!(csf, csf.clone());
    }
}
