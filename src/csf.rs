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
}
