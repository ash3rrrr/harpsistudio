use hs_core::engine::Engine;
use hs_core::patterns;

fn main() {
    let sr: f32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(12_000.0);
    for p in patterns::library() {
        let samples = Engine::render_song(&p.to_song(), sr);
        let active_len = ((p.length_beats() * 60.0 / p.bpm()) * sr as f64) as usize;
        let active = &samples[..active_len.min(samples.len())];
        let rms = (active.iter().map(|s| s * s).sum::<f32>() / active.len().max(1) as f32).sqrt();
        let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        let win = (sr as usize / 2).max(1);
        let mut short_rms = rms;
        if active.len() > win {
            let mut energy: f32 = active[..win].iter().map(|s| s * s).sum();
            let mut best = energy;
            for i in win..active.len() {
                energy += active[i] * active[i] - active[i - win] * active[i - win];
                best = best.max(energy);
            }
            short_rms = (best / win as f32).sqrt();
        }
        println!(
            "{}\t{}\t{:.3}\t{:.3}\t{:.3}\t{:.3}",
            p.name,
            p.genre.tag(),
            20.0 * short_rms.max(1e-9).log10(),
            20.0 * rms.max(1e-9).log10(),
            20.0 * peak.max(1e-9).log10(),
            short_rms
        );
    }
}
