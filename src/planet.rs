//! Planète en dalles cubiques : 384 dalles (6 faces × 8 × 8) éclairées par le soleil, qui
//! s'allument une à une avec la progression de la vague.
use crate::draw::{mix, shade, theme, SUN};
use crate::game::*;
use crate::sphere::*;
use macroquad::prelude::*;
use std::f32::consts::FRAC_PI_4;

pub const CELLS: usize = 8; // dalles par côté de face du cube

/// Direction (unitaire) du point (u, v) de la face (`axis`, `sign`) du cube.
fn cube_point(axis: usize, sign: f32, u: f32, v: f32) -> Vec3 {
    let mut p = [0.0; 3];
    p[axis] = sign;
    p[(axis + 1) % 3] = u;
    p[(axis + 2) % 3] = v;
    Vec3::from(p).normalize()
}

/// Arête numéro `k` de la grille d'une face, dans [-1, 1]. La tangente répartit les angles
/// régulièrement : les dalles du centre et des coins ont à peu près la même taille.
fn edge(k: usize) -> f32 {
    ((k as f32 / CELLS as f32 * 2.0 - 1.0) * FRAC_PI_4).tan()
}

pub struct Tile {
    pub corners: [Vec3; 4], // directions unitaires
    pub center: Vec3,
    pub rank: f32, // ordre d'allumage dans [0, 1)
    pub tint: f32, // variation de teinte propre à la dalle
}

/// Danger en un point de la planète (direction unitaire `p`), de 0 à 1 : 1 sous une zone d'impact, et près
/// d'un anneau d'onde qui blesse (décroît linéairement jusqu'à 2,4 de l'anneau).
pub fn danger(g: &Game, p: Vec3) -> f32 {
    if g.meteors.iter().any(|m| arc(p, m.dir) < BLAST + 0.8) {
        return 1.0;
    }
    g.shocks
        .iter()
        .filter(|s| s.harmful)
        .map(|s| (1.0 - (arc(p, s.dir) - s.r).abs() / 2.4).clamp(0.0, 1.0))
        .fold(0.0, f32::max)
}

pub struct Planet {
    pub tiles: Vec<Tile>,
    pub lit: Vec<f32>,   // allumage de chaque dalle (0 à 1)
    pub flash: Vec<f32>, // éclat blanc au moment de l'allumage
    seams: Vec<(Vec3, Vec3)>,
}

impl Planet {
    pub fn new() -> Self {
        let mut rng = Rng::new(2024);
        let (mut tiles, mut seams) = (vec![], vec![]);
        for axis in 0..3 {
            for sign in [-1.0f32, 1.0] {
                for i in 0..CELLS {
                    for j in 0..CELLS {
                        let p = |a: usize, b: usize| cube_point(axis, sign, edge(a), edge(b));
                        let corners = [p(i, j), p(i + 1, j), p(i + 1, j + 1), p(i, j + 1)];
                        let center = corners.iter().sum::<Vec3>().normalize();
                        tiles.push(Tile { corners, center, rank: 0.0, tint: rng.range(-0.06, 0.06) });
                        // joints : arêtes « bas » et « gauche » de chaque dalle, plus « haut » et « droite » en bordure
                        seams.push((corners[0], corners[1]));
                        seams.push((corners[0], corners[3]));
                        if j == CELLS - 1 {
                            seams.push((corners[3], corners[2]));
                        }
                        if i == CELLS - 1 {
                            seams.push((corners[1], corners[2]));
                        }
                    }
                }
            }
        }
        // ordre d'allumage : permutation aléatoire fixe
        let n = tiles.len();
        let mut order: Vec<usize> = (0..n).collect();
        for k in (1..n).rev() {
            order.swap(k, (rng.range(0.0, (k + 1) as f32) as usize).min(k));
        }
        for (rank, &t) in order.iter().enumerate() {
            tiles[t].rank = rank as f32 / n as f32;
        }
        Planet { lit: vec![0.0; n], flash: vec![0.0; n], tiles, seams }
    }

    /// Les dalles s'allument (fondu + éclat) proportionnellement à la progression de la vague.
    pub fn update(&mut self, g: &Game, dt: f32) {
        let progress = if g.lives == 0 { 0.0 } else { (g.got as f32 / cfg(g.wave).quota as f32).min(1.0) };
        let k = 1.0 - (-6.0 * dt).exp();
        for (i, t) in self.tiles.iter().enumerate() {
            let target = if t.rank < progress { 1.0 } else { 0.0 };
            if target == 1.0 && self.lit[i] < 0.02 {
                self.flash[i] = 1.0; // vient de s'allumer
            }
            self.lit[i] += (target - self.lit[i]) * k;
            self.flash[i] = (self.flash[i] - 2.5 * dt).max(0.0);
        }
    }

    pub fn draw(&self, g: &Game) {
        let (sun, t) = (SUN.normalize(), theme(g.wave));
        let progress = (g.got as f32 / cfg(g.wave).quota as f32).min(1.0);
        let (panel_dark, frame_dark) = (Color::new(0.13, 0.08, 0.28, 1.0), Color::new(0.055, 0.04, 0.13, 1.0));
        // trois couches par dalle : cadre sombre, panneau, et pour les dalles allumées un cœur lumineux
        let (mut frames, mut panels, mut cores) = (vec![], vec![], vec![]);
        let (mut frame_idx, mut panel_idx, mut core_idx) = (vec![], vec![], vec![]);
        for (i, tile) in self.tiles.iter().enumerate() {
            // les dalles s'assombrissent autour des ondes et sous les zones d'impact : le danger ressort
            // même sur une planète très allumée
            let danger = danger(g, tile.center);
            // l'onde de fin de vague (cyan) balaie les dalles sans rien assombrir
            let cool = g
                .shocks
                .iter()
                .filter(|s| !s.harmful)
                .map(|s| (1.0 - (arc(tile.center, s.dir) - s.r).abs()).max(0.0) * (1.0 - s.r / (2.0 * SHOCK_MAX)).clamp(0.2, 1.0))
                .fold(0.0f32, f32::max);
            let (lit, flash) = (self.lit[i] * (1.0 - 0.85 * danger), self.flash[i] * (1.0 - danger));
            let dim = 1.0 - 0.4 * danger;
            let shimmer = 0.75 + 0.25 * (0.5 + 0.5 * (g.t * 2.0 + i as f32 * 0.7).sin());
            let tint = |c: Color| mix(mix(c, WHITE, flash * 0.6), CYAN, cool * 0.5);
            let day = |p: Vec3| 0.55 + 0.45 * p.dot(sun).max(0.0); // éclairage du soleil : côté nuit encore lisible
            let (base, core_base) = (frames.len() as u16, cores.len() as u16);
            for &corner in &tile.corners {
                // cadre au niveau de la sphère, panneau légèrement surélevé, cœur encore un peu plus haut
                let toward = |k: f32| (corner + (tile.center - corner) * k).normalize();
                let (mid, core) = (toward(0.08), toward(0.45));
                let frame = shade(frame_dark, day(corner) * dim);
                frames.push(Vertex::new2(corner * R, Vec2::ZERO, mix(frame, CYAN, cool * 0.4)));
                let unlit = shade(panel_dark, day(mid) * (1.0 + tile.tint) * dim);
                let panel = mix(unlit, shade(t, 0.42 * shimmer * (0.7 + 0.3 * day(mid))), lit);
                panels.push(Vertex::new2(mid * (R + 0.03), Vec2::ZERO, tint(panel)));
                if lit > 0.02 {
                    let glow = mix(shade(panel_dark, day(core)), shade(t, shimmer * (0.8 + 0.2 * day(core))), lit);
                    cores.push(Vertex::new2(core * (R + 0.05), Vec2::ZERO, tint(glow)));
                }
            }
            for idx in [0, 1, 2, 0, 2, 3] {
                frame_idx.push(base + idx);
                panel_idx.push(base + idx);
                if lit > 0.02 {
                    core_idx.push(core_base + idx);
                }
            }
        }
        draw_mesh(&Mesh { vertices: frames, indices: frame_idx, texture: None });
        draw_mesh(&Mesh { vertices: panels, indices: panel_idx, texture: None });
        if !core_idx.is_empty() {
            draw_mesh(&Mesh { vertices: cores, indices: core_idx, texture: None });
        }
        let seam = Color { a: 0.35 + 0.3 * progress, ..t };
        for &(a, b) in &self.seams {
            draw_line_3d(a * (R + 0.04), b * (R + 0.04), seam);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_planet_is_a_closed_mosaic_of_384_tiles() {
        let p = Planet::new();
        assert_eq!(p.tiles.len(), 6 * CELLS * CELLS);
        // un quadrilatère = deux triangles ; les dalles étant plates, la somme est légèrement sous 4πR²
        let area: f32 = p
            .tiles
            .iter()
            .map(|t| {
                let [a, b, c, d] = t.corners.map(|v| v * R);
                0.5 * ((b - a).cross(c - a).length() + (c - a).cross(d - a).length())
            })
            .sum();
        let sphere = 4.0 * std::f32::consts::PI * R * R;
        assert!((area / sphere - 1.0).abs() < 0.03, "aire {area} contre {sphere}");
    }

    #[test]
    fn every_tile_has_its_own_lighting_rank() {
        let p = Planet::new();
        let n = p.tiles.len();
        let mut ranks: Vec<u32> = p.tiles.iter().map(|t| (t.rank * n as f32).round() as u32).collect();
        ranks.sort();
        ranks.dedup();
        assert_eq!(ranks.len(), n);
    }

    #[test]
    fn tiles_darken_under_impact_zones_and_near_harmful_shock_rings_only() {
        let mut g = Game::new(1);
        assert_eq!(danger(&g, Vec3::Y), 0.0);
        g.meteors.push(Meteor { dir: Vec3::Y, alt: 10.0, spin: 0.0 });
        assert_eq!(danger(&g, Vec3::Y), 1.0); // sous la zone
        assert_eq!(danger(&g, -Vec3::Y), 0.0); // à l'opposé
        g.meteors.clear();
        let (mut ring_p, mut f) = (Vec3::Y, -Vec3::Z);
        walk(&mut ring_p, &mut f, 6.0); // un point sur l'anneau d'une onde de rayon 6
        g.shocks.push(Shock { dir: Vec3::Y, r: 6.0, harmful: true });
        assert!(danger(&g, ring_p) > 0.99);
        let mut far_p = ring_p;
        walk(&mut far_p, &mut f, 2.5); // plus loin que 2,4 de l'anneau
        assert_eq!(danger(&g, far_p), 0.0);
        g.shocks[0].harmful = false; // l'onde de fin de vague n'assombrit rien
        assert_eq!(danger(&g, ring_p), 0.0);
    }

    #[test]
    fn lit_fraction_follows_the_wave_progress() {
        let mut p = Planet::new();
        let mut g = Game::new(1);
        g.got = cfg(1).quota / 2;
        for _ in 0..20 {
            p.update(&g, 0.1);
        }
        let lit = p.lit.iter().filter(|&&l| l > 0.5).count() as f32 / p.lit.len() as f32;
        assert!((lit - 0.5).abs() < 0.02, "fraction allumée {lit}");
        g.got = 0;
        for _ in 0..20 {
            p.update(&g, 0.1);
        }
        assert!(p.lit.iter().all(|&l| l < 0.5));
    }
}
