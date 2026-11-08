//! Sons générés par le code : chaque son est calculé échantillon par échantillon, emballé dans un WAV en
//! mémoire puis confié à macroquad (fonctionnalité `audio`). La simulation ne joue rien : elle remplit
//! `Game.sfx`, que `Sounds::update` joue après chaque pas.
use crate::game::{Game, Rng, Sfx};
use macroquad::audio::{PlaySoundParams, Sound, load_sound_from_bytes, play_sound};
use std::f32::consts::TAU;

const RATE: u32 = 44_100; // la fréquence du mélangeur de quad-snd : aucun rééchantillonnage
const SECS: f32 = 1.0 / RATE as f32;

/// WAV PCM 16 bits mono. Le décodeur de quad-snd plante sur un fichier mal formé : l'en-tête est testé.
fn wav(samples: &[f32]) -> Vec<u8> {
    let data = 2 * samples.len() as u32;
    let mut b = Vec::with_capacity(44 + data as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes()); // taille du bloc « fmt »
    b.extend_from_slice(&1u16.to_le_bytes()); // PCM
    b.extend_from_slice(&1u16.to_le_bytes()); // mono
    b.extend_from_slice(&RATE.to_le_bytes());
    b.extend_from_slice(&(2 * RATE).to_le_bytes()); // octets par seconde
    b.extend_from_slice(&2u16.to_le_bytes()); // octets par échantillon
    b.extend_from_slice(&16u16.to_le_bytes()); // bits par échantillon
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data.to_le_bytes());
    for s in samples {
        b.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    b
}

/// `secs` secondes de la forme `shape` (une période pour une phase de 0 à 1) à la fréquence `freq(t)` (Hz,
/// elle peut glisser), multipliées par une enveloppe : attaque de 5 ms, puis volume divisé par e toutes les
/// `decay` secondes.
fn tone(secs: f32, decay: f32, freq: impl Fn(f32) -> f32, shape: fn(f32) -> f32) -> Vec<f32> {
    let mut phase = 0.0f32;
    (0..(secs * RATE as f32) as usize)
        .map(|i| {
            let t = i as f32 * SECS;
            phase = (phase + freq(t) * SECS).fract();
            shape(phase) * (t / 0.005).min(1.0) * (-t / decay).exp()
        })
        .collect()
}

/// Bruit blanc lissé (plus `k` est petit, plus il est sourd), avec la même enveloppe que `tone`.
fn noise(secs: f32, decay: f32, k: f32) -> Vec<f32> {
    let (mut rng, mut y) = (Rng::new(7), 0.0f32);
    (0..(secs * RATE as f32) as usize)
        .map(|i| {
            let t = i as f32 * SECS;
            y += (rng.range(-1.0, 1.0) - y) * k;
            y * (t / 0.005).min(1.0) * (-t / decay).exp()
        })
        .collect()
}

/// Superpose des sons (son, départ en s, volume relatif), chacun ramené d'abord à un maximum de 1, puis le
/// tout à un maximum de 0,9.
fn mix(parts: &[(&[f32], f32, f32)]) -> Vec<f32> {
    let peak = |s: &[f32]| s.iter().fold(1e-6f32, |m, x| m.max(x.abs()));
    let len = parts.iter().map(|(s, at, _)| (at * RATE as f32) as usize + s.len()).max().unwrap_or(0);
    let mut out = vec![0.0; len];
    for &(s, at, gain) in parts {
        let k = gain / peak(s);
        for (o, x) in out[(at * RATE as f32) as usize..].iter_mut().zip(s) {
            *o += x * k;
        }
    }
    let k = 0.9 / peak(&out);
    out.iter().map(|x| x * k).collect()
}

fn sine(p: f32) -> f32 {
    (p * TAU).sin()
}

fn square(p: f32) -> f32 {
    if p < 0.5 { 1.0 } else { -1.0 }
}

fn triangle(p: f32) -> f32 {
    1.0 - 4.0 * (p - 0.5).abs()
}

/// Sinus enrichi de son octave : un timbre de clochette.
fn bell(p: f32) -> f32 {
    sine(p) + 0.3 * (2.0 * p * TAU).sin()
}

/// Do 5 monté de `semi` demi-tons.
fn pitch(semi: f32) -> f32 {
    523.25 * 2f32.powf(semi / 12.0)
}

/// Note de chaîne `i` (0 à 7) : gamme pentatonique, de plus en plus aiguë à mesure que la chaîne s'allonge.
fn note(i: usize) -> Vec<f32> {
    let f = pitch([0.0, 2.0, 4.0, 7.0, 9.0, 12.0, 14.0, 16.0][i.min(7)]);
    mix(&[(&tone(0.3, 0.09, |_| f, bell), 0.0, 1.0)])
}

/// Mauvais cristal : bourdonnement qui descend.
fn wrong() -> Vec<f32> {
    mix(&[(&tone(0.3, 0.12, |t| 230.0 - 260.0 * t, square), 0.0, 1.0)])
}

/// Impact : grondement sourd et coup grave.
fn impact() -> Vec<f32> {
    mix(&[(&noise(0.7, 0.16, 0.06), 0.0, 1.0), (&tone(0.6, 0.18, |t| 85.0 - 50.0 * t, sine), 0.0, 0.8)])
}

/// Coup reçu : note dure qui chute de deux octaves, avec un éclat de bruit.
fn hit() -> Vec<f32> {
    mix(&[(&tone(0.45, 0.14, |t| 520.0 * 0.25f32.powf(t / 0.45), square), 0.0, 1.0), (&noise(0.2, 0.05, 0.5), 0.0, 0.6)])
}

/// Dash : souffle et sifflement qui monte.
fn dash() -> Vec<f32> {
    mix(&[(&noise(0.3, 0.09, 0.3), 0.0, 1.0), (&tone(0.3, 0.12, |t| 300.0 + 3000.0 * t, sine), 0.0, 0.35)])
}

/// Vague terminée : arpège montant (do, mi, sol, do).
fn wave_done() -> Vec<f32> {
    let n = |semi: f32| tone(0.6, 0.18, move |_| pitch(semi), bell);
    mix(&[(&n(0.0), 0.0, 1.0), (&n(4.0), 0.09, 1.0), (&n(7.0), 0.18, 1.0), (&n(12.0), 0.27, 1.2)])
}

/// Drone qui s'arme : sirène qui monte pendant les 1,2 s de l'armement.
fn drone() -> Vec<f32> {
    let rise = tone(1.2, 100.0, |t| 160.0 + 380.0 * t, triangle);
    let end = rise.len() as f32 * SECS;
    let shaped: Vec<f32> = rise
        .iter()
        .enumerate()
        .map(|(i, x)| {
            let t = i as f32 * SECS;
            x * (0.65 + 0.35 * (t * 14.0 * TAU).sin()) * ((end - t) / 0.1).min(1.0) // trémolo, puis fondu
        })
        .collect();
    mix(&[(&shaped, 0.0, 1.0)])
}

/// Onde dans le dos : double bip aigu.
fn behind() -> Vec<f32> {
    let beep = tone(0.07, 1.0, |_| 990.0, square);
    mix(&[(&beep, 0.0, 1.0), (&beep, 0.12, 1.0)])
}

pub struct Sounds {
    chain: Vec<Sound>,
    wrong: Sound,
    impact: Sound,
    hit: Sound,
    dash: Sound,
    wave: Sound,
    drone: Sound,
    behind: Sound,
    behind_cd: f32, // délai avant de pouvoir rejouer l'avertissement
}

impl Sounds {
    /// `None` si un son ne se charge pas : le jeu continue en silence.
    pub async fn load() -> Option<Sounds> {
        let load = |s: Vec<f32>| async move { load_sound_from_bytes(&wav(&s)).await.ok() };
        let mut chain = vec![];
        for i in 0..8 {
            chain.push(load(note(i)).await?);
        }
        Some(Sounds {
            chain,
            wrong: load(wrong()).await?,
            impact: load(impact()).await?,
            hit: load(hit()).await?,
            dash: load(dash()).await?,
            wave: load(wave_done()).await?,
            drone: load(drone()).await?,
            behind: load(behind()).await?,
            behind_cd: 0.0,
        })
    }

    /// Joue les sons du dernier pas, et un double bip quand une onde arrive dans le dos du joueur.
    pub fn update(&mut self, g: &Game, dt: f32) {
        let play = |s: &Sound, volume: f32| play_sound(s, PlaySoundParams { looped: false, volume });
        let mut played = vec![];
        for &e in &g.sfx {
            if played.contains(&e) {
                continue; // un seul exemplaire par image : les six impacts d'une salve ne saturent pas
            }
            played.push(e);
            match e {
                Sfx::Chain(n) => play(&self.chain[(n as usize).clamp(1, 8) - 1], 0.35),
                Sfx::Wrong => play(&self.wrong, 0.4),
                Sfx::Impact => play(&self.impact, 0.3),
                Sfx::Hit => play(&self.hit, 0.55),
                Sfx::Dash => play(&self.dash, 0.3),
                Sfx::Wave => play(&self.wave, 0.45),
                Sfx::Drone => play(&self.drone, 0.25),
            }
        }
        self.behind_cd = (self.behind_cd - dt).max(0.0);
        if self.behind_cd == 0.0 && g.ring_behind() {
            play(&self.behind, 0.4);
            self.behind_cd = 0.6;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_is_a_valid_16_bit_mono_pcm_file() {
        let w = wav(&[0.0, 1.0, -1.0, 2.0]);
        assert_eq!(w.len(), 44 + 8);
        assert_eq!((&w[0..4], &w[8..16], &w[36..40]), (&b"RIFF"[..], &b"WAVEfmt "[..], &b"data"[..]));
        let u32_at = |i: usize| u32::from_le_bytes(w[i..i + 4].try_into().unwrap());
        assert_eq!((u32_at(4), u32_at(16), u32_at(24), u32_at(28), u32_at(40)), (36 + 8, 16, RATE, 2 * RATE, 8));
        let sample = |k: usize| i16::from_le_bytes([w[44 + 2 * k], w[45 + 2 * k]]);
        assert_eq!((sample(0), sample(1), sample(2), sample(3)), (0, 32767, -32767, 32767)); // 2,0 est écrêté
    }

    #[test]
    fn every_sound_is_short_loud_enough_and_never_clips() {
        let all = (0..8).map(note).chain([wrong(), impact(), hit(), dash(), wave_done(), drone(), behind()]);
        for (i, s) in all.enumerate() {
            let peak = s.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(!s.is_empty() && s.len() < 2 * RATE as usize, "son {i} : {} échantillons", s.len());
            assert!((0.5..=0.91).contains(&peak), "son {i} : maximum {peak}");
        }
    }
}
