//! Dessin 3D : primitives (boîtes néon, anneaux, disques), grille de la planète, caméra et scène.
use crate::game::*;
use crate::sphere::*;
use macroquad::prelude::*;

const SUN: Vec3 = vec3(-0.5, 0.7, -0.5); // direction du soleil (non normalisée)

/// Couleur de la planète pour la vague `wave` : change à chaque vague.
pub fn theme(wave: u32) -> Color {
    const T: [(f32, f32, f32); 5] = [(1.0, 0.2, 0.7), (0.25, 0.6, 1.0), (0.3, 1.0, 0.5), (0.7, 0.4, 1.0), (0.2, 1.0, 0.9)];
    let (r, g, b) = T[wave.saturating_sub(1) as usize % 5];
    Color::new(r, g, b, 1.0)
}

pub fn ring(dir: Vec3, rad: f32, lift: f32, col: Color) {
    for w in circle(dir, rad).windows(2) {
        draw_line_3d(w[0] * (R + lift), w[1] * (R + lift), col);
    }
}

/// Anneau plein entre les rayons `r0` et `r1`, posé sur la planète. `rings` couronnes
/// concentriques pour épouser la courbure.
pub fn band(dir: Vec3, r0: f32, r1: f32, rings: u16, lift: f32, col: Color) {
    let (mut vertices, mut indices) = (vec![], vec![]);
    for k in 0..=rings {
        for p in circle(dir, r0 + (r1 - r0) * k as f32 / rings as f32) {
            vertices.push(Vertex::new2(p * (R + lift), Vec2::ZERO, col));
        }
    }
    for k in 0..rings {
        for i in 0..32u16 {
            let (a, c) = (k * 33 + i, (k + 1) * 33 + i);
            indices.extend([a, c, a + 1, a + 1, c, c + 1]);
        }
    }
    draw_mesh(&Mesh { vertices, indices, texture: None });
}

pub fn disc(dir: Vec3, rad: f32, lift: f32, col: Color) {
    band(dir, 0.0, rad, 3, lift, col);
}

/// Boîte orientée (axes `ax`, demi-tailles `half`) : faces ombrées par le soleil + arêtes néon.
pub fn draw_box(c: Vec3, ax: [Vec3; 3], half: Vec3, col: Color) {
    let sun = SUN.normalize();
    let corners = [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
    let (mut vertices, mut indices) = (Vec::with_capacity(24), Vec::with_capacity(36));
    for i in 0..3 {
        let (j, k) = ((i + 1) % 3, (i + 2) % 3);
        for s in [-1.0f32, 1.0] {
            let shade = 0.45 + 0.55 * (ax[i] * s).dot(sun).max(0.0);
            let fill = Color::new(col.r * shade, col.g * shade, col.b * shade, col.a);
            let base = vertices.len() as u16;
            for (sj, sk) in corners {
                let p = c + ax[i] * half[i] * s + ax[j] * half[j] * sj + ax[k] * half[k] * sk;
                vertices.push(Vertex::new2(p, Vec2::ZERO, fill));
            }
            indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }
    draw_mesh(&Mesh { vertices, indices, texture: None });
    // arêtes légèrement dilatées pour ne pas z-fighter avec les faces
    for i in 0..3 {
        let (j, k) = ((i + 1) % 3, (i + 2) % 3);
        let e = ax[i] * half[i] * 1.01;
        for (sj, sk) in corners {
            let p = c + (ax[j] * half[j] * sj + ax[k] * half[k] * sk) * 1.01;
            draw_line_3d(p - e, p + e, col);
        }
    }
}

/// Grille « cube-sphère » de la planète (calculée une seule fois) : contrairement à
/// méridiens/parallèles, aucune ligne ne se pince aux pôles.
pub fn planet_grid() -> Vec<(Vec3, Vec3)> {
    let (cells, steps) = (6, 12);
    let mut g = vec![];
    for axis in 0..3 {
        for sign in [-1.0f32, 1.0] {
            for line in 0..=cells {
                let a = line as f32 / cells as f32 * 2.0 - 1.0;
                for along_u in [false, true] {
                    let at = |s: i32| {
                        let b = s as f32 / steps as f32 * 2.0 - 1.0;
                        let mut v = [0.0; 3];
                        v[axis] = sign;
                        v[(axis + 1) % 3] = if along_u { b } else { a };
                        v[(axis + 2) % 3] = if along_u { a } else { b };
                        Vec3::from(v).normalize() * R * 1.004
                    };
                    g.extend((0..steps).map(|s| (at(s), at(s + 1))));
                }
            }
        }
    }
    g
}

impl Game {
    /// Caméra à la troisième personne, lissée, avec « haut » = verticale du joueur.
    pub fn camera(&mut self, dt: f32) -> Camera3D {
        let k = 1.0 - (-6.0 * dt).exp();
        self.cam = self.cam.lerp(self.cam_goal(), k);
        self.cam_up = self.cam_up.lerp(self.n, k).normalize();
        self.shake = (self.shake - 2.0 * dt).max(0.0);
        let jitter = vec3(rand::gen_range(-1.0, 1.0), rand::gen_range(-1.0, 1.0), rand::gen_range(-1.0, 1.0));
        Camera3D {
            position: self.cam + jitter * self.shake * 0.4,
            target: self.pos() + self.f * 2.0,
            up: self.cam_up,
            fovy: 1.0 + 0.15 * self.dash_t / DASH_TIME, // petit coup de zoom pendant le dash
            z_near: 0.5,
            z_far: 1000.0,
            ..Default::default()
        }
    }

    pub fn draw(&self, stars: &[Vec3], grid: &[(Vec3, Vec3)]) {
        // ciel
        for &s in stars {
            draw_cube(s, Vec3::splat(1.2), None, WHITE);
        }
        let smooth = |rings, slices| DrawSphereParams { rings, slices, ..Default::default() };
        let sun = SUN.normalize() * 260.0;
        draw_sphere_ex(sun, 22.0, None, Color::new(1.0, 0.62, 0.2, 1.0), smooth(24, 48));
        draw_sphere_ex(sun, 34.0, None, Color::new(1.0, 0.4, 0.3, 0.18), smooth(24, 48)); // halo, dessiné après le disque

        // planète : sa couleur change à chaque vague, la grille s'illumine avec le quota
        let t = theme(self.wave);
        let progress = (self.got as f32 / cfg(self.wave).quota as f32).min(1.0);
        let fill = Color::new(0.09 + 0.07 * t.r, 0.045 + 0.07 * t.g, 0.195 + 0.07 * t.b, 1.0);
        draw_sphere_ex(Vec3::ZERO, R, None, fill, smooth(40, 64));
        let lines = Color { a: 0.3 + 0.55 * progress, ..t };
        for &(a, b) in grid {
            draw_line_3d(a, b, lines);
        }

        // marquages au sol : aura du joueur, zones d'impact, ondes, traînée
        disc(self.n, PICKUP, 0.05, Color { a: 0.06, ..CYAN });
        ring(self.n, PICKUP, 0.06, Color { a: 0.5, ..CYAN });
        for m in &self.meteors {
            let col = if arc(m.dir, self.n) < BLAST { Color::new(1.0, 0.15, 0.1, 1.0) } else { FIRE };
            disc(m.dir, BLAST, 0.03, Color { a: 0.18, ..col });
            ring(m.dir, BLAST, 0.06, col);
            ring(m.dir, BLAST * (1.0 - (m.alt / ALT0).min(1.0)), 0.06, col); // se remplit jusqu'à l'impact
            draw_line_3d(m.dir * R, m.dir * (R + m.alt), Color { a: 0.3, ..col }); // trajectoire de chute
        }
        for s in &self.shocks {
            let (col, max) = if s.harmful { (FIRE, SHOCK_MAX) } else { (CYAN, 2.0 * SHOCK_MAX) };
            let a = (1.0 - s.r / max).clamp(0.25, 1.0);
            // bande de 1,2 de large (la zone qui blesse), liseré blanc au centre
            band(s.dir, (s.r - 0.6).max(0.0), s.r + 0.6, 2, 0.08, Color { a: 0.5 * a, ..col });
            ring(s.dir, s.r, 0.09, Color { a, ..WHITE });
        }
        let pts: Vec<Vec3> = self.trail.iter().copied().chain([self.n]).collect();
        let (mut vertices, mut indices) = (vec![], vec![]);
        for (i, &p) in pts.iter().enumerate() {
            let dir = pts[(i + 1).min(pts.len() - 1)] - pts[i.saturating_sub(1)];
            let side = dir.cross(p).normalize_or_zero() * 0.1;
            let col = Color { a: 0.6 * i as f32 / pts.len() as f32, ..CYAN };
            vertices.push(Vertex::new2(p * (R + 0.04) + side, Vec2::ZERO, col));
            vertices.push(Vertex::new2(p * (R + 0.04) - side, Vec2::ZERO, col));
            if i > 0 {
                let b = 2 * i as u16;
                indices.extend([b - 2, b - 1, b, b - 1, b + 1, b]);
            }
        }
        draw_mesh(&Mesh { vertices, indices, texture: None });

        // cristaux (avec un faisceau visible depuis l'autre côté de l'horizon) ; ils clignotent avant de disparaître
        for (i, k) in self.crystals.iter().enumerate() {
            if k.life < 3.0 && (self.t * 8.0) as i32 % 2 == 0 {
                continue;
            }
            let (col, size, spin) = if k.gold { (AMBER, 1.25, 4.0) } else { (CYAN, 1.0, 2.0) };
            let c = k.dir;
            let bob = (self.t * 3.0 + i as f32 * 2.0).sin() * 0.15;
            let q = Quat::from_axis_angle(c, self.t * spin + i as f32);
            let u = c.any_orthonormal_vector();
            draw_line_3d(c * R, c * (R + 12.0), Color { a: 0.35, ..col });
            draw_box(c * (R + 1.7 + bob), [q * u, c, q * c.cross(u)], vec3(0.36, 0.8, 0.36) * size, col);
        }

        // météorites
        for m in &self.meteors {
            let p = m.dir * (R + m.alt);
            let q = Quat::from_euler(EulerRot::XYZ, self.t * 2.1 + m.spin, self.t * 1.3 + m.spin * 2.0, m.spin);
            draw_line_3d(p, p + m.dir * 6.0, FIRE);
            draw_box(p, [q * Vec3::X, q * Vec3::Y, q * Vec3::Z], Vec3::splat(1.2), FIRE);
        }

        // drones : clignotent en blanc tant qu'ils s'arment ; l'anneau rouge est leur zone de contact
        for d in &self.drones {
            let ready = d.arm <= 0.0;
            let col = if ready || (self.t * 12.0) as i32 % 2 == 0 { DRONE } else { WHITE };
            let p = d.n * (R + 0.8 + 0.1 * (self.t * 5.0).sin());
            let ax = [d.f.cross(d.n), d.n, d.f];
            let q = Quat::from_axis_angle(d.n, self.t * 8.0);
            ring(d.n, 0.9, 0.06, Color { a: if ready { 0.7 } else { 0.25 }, ..DRONE });
            draw_box(p, ax, vec3(0.5, 0.28, 0.5), col);
            draw_box(p + d.n * 0.38, [q * ax[0], ax[1], q * ax[2]], vec3(0.75, 0.04, 0.1), WHITE); // hélice
        }

        // astronaute (clignote quand il est invulnérable)
        if self.hurt == 0.0 || (self.t * 14.0) as i32 % 2 == 0 {
            let (n, f, p) = (self.n, self.f, self.pos());
            let ax = [f.cross(n), n, f];
            let (suit, s) = (Color::new(0.85, 0.9, 1.0, 1.0), 1.3); // s : échelle de l'astronaute
            draw_box(p + n * 0.45 * s, ax, vec3(0.28, 0.4, 0.2) * s, suit);
            draw_box(p + n * 1.05 * s, ax, vec3(0.2, 0.18, 0.2) * s, suit);
            draw_box(p + n * 1.07 * s + f * 0.17 * s, ax, vec3(0.15, 0.07, 0.06) * s, CYAN);
            draw_box(p + n * 0.55 * s - f * 0.28 * s, ax, vec3(0.17, 0.26, 0.1) * s, HOTPINK);
        }

        for s in &self.sparks {
            draw_cube(s.pos, Vec3::splat(0.1 + 0.15 * s.life), None, Color { a: (s.life * 2.0).min(1.0), ..s.col });
        }
    }
}
