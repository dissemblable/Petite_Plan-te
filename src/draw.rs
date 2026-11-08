//! Dessin 3D : primitives (boîtes néon, anneaux, disques, halos), caméra et scène complète.
use crate::fx;
use crate::game::*;
use crate::models;
use crate::planet::Planet;
use crate::sphere::*;
use macroquad::prelude::*;

pub const SUN: Vec3 = vec3(-0.5, 0.7, -0.5); // direction du soleil (non normalisée)

/// Couleur de la planète pour la vague `wave` : change à chaque vague.
pub fn theme(wave: u32) -> Color {
    const T: [(f32, f32, f32); 5] = [(1.0, 0.2, 0.7), (0.25, 0.6, 1.0), (0.3, 1.0, 0.5), (0.7, 0.4, 1.0), (0.2, 1.0, 0.9)];
    let (r, g, b) = T[wave.saturating_sub(1) as usize % 5];
    Color::new(r, g, b, 1.0)
}

/// Mélange linéaire de deux couleurs.
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    Color::new(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, a.a + (b.a - a.a) * t)
}

/// Couleur dont le rouge, le vert et le bleu sont multipliés par `k`.
pub fn shade(c: Color, k: f32) -> Color {
    Color::new(c.r * k, c.g * k, c.b * k, c.a)
}

pub fn ring(dir: Vec3, rad: f32, lift: f32, col: Color) {
    for w in circle(dir, rad).windows(2) {
        draw_line_3d(w[0] * (R + lift), w[1] * (R + lift), col);
    }
}

/// Anneau pointillé : huit tirets qui défilent avec `phase` (fourmis qui marchent).
pub fn dashed_ring(dir: Vec3, rad: f32, lift: f32, col: Color, phase: f32) {
    for (i, w) in circle(dir, rad).windows(2).enumerate() {
        if ((i as f32 + phase).floor() as i32).rem_euclid(4) < 2 {
            draw_line_3d(w[0] * (R + lift), w[1] * (R + lift), col);
        }
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

fn box_ex(c: Vec3, ax: [Vec3; 3], half: Vec3, fill: Color, edge: Color, shaded: bool) {
    let sun = SUN.normalize();
    let corners = [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
    let (mut vertices, mut indices) = (Vec::with_capacity(24), Vec::with_capacity(36));
    for i in 0..3 {
        let (j, k) = ((i + 1) % 3, (i + 2) % 3);
        for s in [-1.0f32, 1.0] {
            let light = if shaded { 0.45 + 0.55 * (ax[i] * s).dot(sun).max(0.0) } else { 1.0 };
            let col = shade(fill, light);
            let base = vertices.len() as u16;
            for (sj, sk) in corners {
                let p = c + ax[i] * half[i] * s + ax[j] * half[j] * sj + ax[k] * half[k] * sk;
                vertices.push(Vertex::new2(p, Vec2::ZERO, col));
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
            draw_line_3d(p - e, p + e, edge);
        }
    }
}

/// Boîte orientée (axes `ax`, demi-tailles `half`) : faces ombrées par le soleil, arêtes néon.
pub fn draw_box(c: Vec3, ax: [Vec3; 3], half: Vec3, fill: Color, edge: Color) {
    box_ex(c, ax, half, fill, edge, true);
}

/// Boîte émissive : faces sans ombrage (elles rayonnent avec la lueur), arêtes plus claires.
pub fn glow_box(c: Vec3, ax: [Vec3; 3], half: Vec3, col: Color) {
    box_ex(c, ax, half, col, mix(col, WHITE, 0.6), false);
}

/// Halo dessiné en dernier, face à la caméra.
pub struct Sprite {
    pub pos: Vec3,
    pub size: f32,
    pub col: Color,
}

pub struct Scene {
    stars: Vec<Vec3>,
    pub planet: Planet,
    glow: Texture2D,
    sprite_mat: Option<Material>,
    bloom: Option<fx::Bloom>,
    pub bloom_on: bool, // touche G
    prev_f: Vec3,
    pub roll: f32, // roulis de la caméra dans les virages
}

impl Scene {
    pub fn new(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        Scene {
            stars: (0..300).map(|_| rng.dir() * 300.0).collect(),
            planet: Planet::new(),
            glow: fx::glow_texture(),
            sprite_mat: fx::sprite_material(),
            bloom: fx::Bloom::new(),
            bloom_on: true,
            prev_f: -Vec3::Z,
            roll: 0.0,
        }
    }

    /// Post-traitement : lueur néon (si disponible et activée) puis vignettes. À appeler après `draw`
    /// et avant l'interface.
    pub fn post(&mut self) {
        set_default_camera();
        if let (true, Some(bloom)) = (self.bloom_on, self.bloom.as_mut()) {
            bloom.apply();
        }
        fx::vignette();
    }

    pub fn update(&mut self, g: &Game, dt: f32) {
        self.planet.update(g, dt);
        // roulis : suit la vitesse de rotation du regard (> 0 : virage à gauche)
        let rate = if dt > 0.0 { self.prev_f.cross(g.f).dot(g.n) / dt } else { 0.0 };
        self.prev_f = g.f;
        self.roll += (rate.clamp(-3.0, 3.0) * 0.02 - self.roll) * (1.0 - (-5.0 * dt).exp());
    }

    pub fn draw(&self, g: &Game, cam: &Camera3D) {
        // ciel
        for &s in &self.stars {
            draw_cube(s, Vec3::splat(1.2), None, WHITE);
        }
        let smooth = |rings, slices| DrawSphereParams { rings, slices, ..Default::default() };
        let sun = SUN.normalize() * 260.0;
        draw_sphere_ex(sun, 22.0, None, Color::new(1.0, 0.62, 0.2, 1.0), smooth(24, 48));
        draw_sphere_ex(sun, 34.0, None, Color::new(1.0, 0.4, 0.3, 0.18), smooth(24, 48)); // halo, dessiné après le disque

        self.planet.draw(g);
        let mut sprites = vec![];

        // Marquages au sol. Les décalques translucides écrivent la profondeur (miniquad n'active le test de
        // profondeur que dans ce cas ; sans lui on les verrait à travers la planète) : on les dessine donc du
        // plus bas au plus haut, au-dessus des dalles (cœurs à 0,05).
        // Zone dangereuse à l'impact : le souffle, plus (dès la vague 2) le liseré où l'onde blesse dès sa naissance.
        // Elle passe au rouge quand le joueur est dedans.
        let reach = if cfg(g.wave).shocks { BLAST + SHOCK_BAND } else { BLAST };
        let zone = |m: &Meteor| if arc(m.dir, g.n) < reach { Color::new(1.0, 0.15, 0.1, 1.0) } else { FIRE };
        for m in &g.meteors {
            self.additive(|| {
                disc(m.dir, BLAST, 0.065, Color { a: 0.3, ..zone(m) }); // zone d'impact
                if reach > BLAST {
                    band(m.dir, BLAST, reach, 1, 0.065, Color { a: 0.14, ..zone(m) }); // liseré de l'onde
                }
            });
        }
        let pts: Vec<Vec3> = g.trail.iter().map(|&(p, _)| p).chain([g.n]).collect();
        let (mut vertices, mut indices) = (vec![], vec![]);
        for (i, &p) in pts.iter().enumerate() {
            let dir = pts[(i + 1).min(pts.len() - 1)] - pts[i.saturating_sub(1)];
            let side = dir.cross(p).normalize_or_zero() * 0.1;
            let col = if i < g.trail.len() && g.trail_hot(i) {
                Color { a: 0.9, ..DRONE } // dalles fragiles : la traînée brûle
            } else {
                Color { a: 0.6 * i as f32 / pts.len() as f32, ..CYAN }
            };
            vertices.push(Vertex::new2(p * (R + 0.07) + side, Vec2::ZERO, col));
            vertices.push(Vertex::new2(p * (R + 0.07) - side, Vec2::ZERO, col));
            if i > 0 {
                let b = 2 * i as u16;
                indices.extend([b - 2, b - 1, b, b - 1, b + 1, b]);
            }
        }
        draw_mesh(&Mesh { vertices, indices, texture: None }); // traînée du joueur
        let aura = KINDS[g.need]; // couleur du prochain cristal à prendre (cyan avant la vague 5)
        disc(g.n, PICKUP, 0.075, Color { a: 0.06, ..aura }); // aura du joueur
        // lignes : aura (anneau pointillé et repères aux quatre coins), contours des zones, trajectoires
        dashed_ring(g.n, PICKUP, 0.09, Color { a: 0.8, ..aura }, g.t * 6.0);
        let (near, far) = (circle(g.n, PICKUP * 0.8), circle(g.n, PICKUP * 1.3));
        for i in [0, 8, 16, 24] {
            draw_line_3d(near[i] * (R + 0.09), far[i] * (R + 0.09), aura);
        }
        for m in &g.meteors {
            let col = zone(m);
            if reach > BLAST {
                ring(m.dir, reach, 0.09, Color { a: 0.3, ..col }); // bord du liseré
            }
            ring(m.dir, BLAST, 0.09, Color { a: 0.6, ..col });
            dashed_ring(m.dir, BLAST, 0.095, col, -g.t * 5.0);
            ring(m.dir, BLAST * (1.0 - (m.alt / ALT0).min(1.0)), 0.09, col); // se remplit jusqu'à l'impact
            draw_line_3d(m.dir * R, m.dir * (R + m.alt), Color { a: 0.3, ..col }); // trajectoire de chute
        }
        for s in &g.shocks {
            let max = if s.harmful { SHOCK_MAX } else { 2.0 * SHOCK_MAX };
            let a = (1.0 - s.r / max).clamp(0.25, 1.0);
            if s.harmful {
                // additif : la bande ajoute de la lumière quelle que soit la couleur des dalles dessous.
                // Halo large (avertissement), zone qui blesse (2 × SHOCK_BAND de large), cœur blanc chaud.
                self.additive(|| {
                    band(s.dir, (s.r - 1.0).max(0.0), s.r + 1.0, 2, 0.10, Color::new(1.0, 0.35, 0.05, 0.4 * a));
                    band(s.dir, (s.r - SHOCK_BAND).max(0.0), s.r + SHOCK_BAND, 2, 0.105, Color::new(1.0, 0.65, 0.2, 0.85 * a));
                    band(s.dir, (s.r - 0.2).max(0.0), s.r + 0.2, 1, 0.11, Color::new(1.0, 0.95, 0.75, a));
                });
            } else {
                band(s.dir, (s.r - 0.6).max(0.0), s.r + 0.6, 2, 0.10, Color { a: 0.5 * a, ..CYAN });
            }
            ring(s.dir, s.r, 0.12, Color { a, ..WHITE });
            if s.harmful && s.r < BLAST + 1.6 {
                let f = 1.0 - (s.r - BLAST) / 1.6; // éclair de l'impact
                sprites.push(Sprite { pos: s.dir * (R + 0.4), size: 5.0 + 2.0 * (1.0 - f), col: Color::new(1.0, 0.75, 0.4, f) });
            }
        }

        // objets
        for (i, k) in g.crystals.iter().enumerate() {
            models::crystal(g, i, k, &mut sprites);
        }
        for m in &g.meteors {
            models::meteor(g, m, &mut sprites);
        }
        for d in &g.drones {
            models::drone(g, d, &mut sprites);
        }
        models::astronaut(g, &mut sprites);
        for s in &g.sparks {
            draw_cube(s.pos, Vec3::splat(0.1 + 0.15 * s.life), None, Color { a: (s.life * 2.0).min(1.0), ..s.col });
        }

        // halos, en dernier et du plus loin au plus proche (ils écrivent la profondeur, comme les décalques)
        sprites.sort_by(|a, b| (b.pos - cam.position).length_squared().total_cmp(&(a.pos - cam.position).length_squared()));
        let fwd = (cam.target - cam.position).normalize();
        let right = fwd.cross(cam.up).normalize();
        self.draw_sprites(&sprites, right, right.cross(fwd));
    }

    /// Exécute `f` avec le matériau additif des halos : la lumière s'ajoute à ce qui est dessous. Valable
    /// pour des triangles seulement (les lignes ne passent pas par un matériau personnalisé).
    fn additive(&self, f: impl FnOnce()) {
        if let Some(m) = &self.sprite_mat {
            gl_use_material(m);
        }
        f();
        gl_use_default_material();
    }

    fn draw_sprites(&self, sprites: &[Sprite], right: Vec3, up: Vec3) {
        if sprites.is_empty() {
            return;
        }
        let (mut vertices, mut indices) = (Vec::with_capacity(4 * sprites.len()), Vec::with_capacity(6 * sprites.len()));
        for (i, s) in sprites.iter().enumerate() {
            let (r, u) = (right * s.size, up * s.size);
            let base = 4 * i as u16;
            for (corner, uv) in [(-r - u, vec2(0.0, 0.0)), (r - u, vec2(1.0, 0.0)), (r + u, vec2(1.0, 1.0)), (-r + u, vec2(0.0, 1.0))] {
                vertices.push(Vertex::new2(s.pos + corner, uv, s.col));
            }
            indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        if let Some(m) = &self.sprite_mat {
            gl_use_material(m);
        }
        draw_mesh(&Mesh { vertices, indices, texture: Some(self.glow.clone()) });
        gl_use_default_material();
    }
}

impl Game {
    /// Caméra à la troisième personne, lissée, avec « haut » = verticale du joueur. `roll` penche
    /// légèrement l'image dans les virages (> 0 : à gauche).
    pub fn camera(&mut self, dt: f32, roll: f32) -> Camera3D {
        let k = 1.0 - (-6.0 * dt).exp();
        self.cam = self.cam.lerp(self.cam_goal(), k);
        self.cam_up = self.cam_up.lerp(self.n, k).normalize();
        self.shake = (self.shake - 2.0 * dt).max(0.0);
        let jitter = vec3(rand::gen_range(-1.0, 1.0), rand::gen_range(-1.0, 1.0), rand::gen_range(-1.0, 1.0));
        let (position, target) = (self.cam + jitter * self.shake * 0.4, self.pos() + self.f * 0.5);
        let right = (target - position).normalize().cross(self.cam_up).normalize();
        Camera3D {
            position,
            target,
            up: (self.cam_up - right * roll).normalize(),
            fovy: 1.0 + 0.15 * self.dash_t / DASH_TIME, // petit coup de zoom pendant le dash
            z_near: 0.5,
            z_far: 1000.0,
            ..Default::default()
        }
    }
}
