//! Petite Planète : un mini-jeu 3D (macroquad).
//! Cours sur une planète, ramasse les cristaux, esquive les météorites.
use macroquad::prelude::*;
use std::f32::consts::TAU;

const R: f32 = 9.0; // rayon de la planète
const BLAST: f32 = 3.0; // rayon de souffle d'une météorite
const ALT0: f32 = 26.0; // altitude de départ des météorites
const FALL: f32 = 12.0; // vitesse de chute des météorites
const PICKUP: f32 = 1.6; // rayon de ramassage d'un cristal
const SUN: Vec3 = vec3(-0.5, 0.7, -0.5); // direction du soleil (non normalisée)
const CYAN: Color = Color::new(0.15, 0.95, 1.0, 1.0);
const PINK: Color = Color::new(1.0, 0.2, 0.7, 1.0);
const FIRE: Color = Color::new(1.0, 0.5, 0.1, 1.0);

// ---------- géométrie sur la sphère ----------

/// Direction unitaire uniforme sur la sphère.
fn rand_dir() -> Vec3 {
    let z: f32 = rand::gen_range(-1.0, 1.0);
    let a: f32 = rand::gen_range(0.0, TAU);
    let r = (1.0 - z * z).sqrt();
    vec3(r * a.cos(), z, r * a.sin())
}

/// Distance à la surface entre deux directions unitaires.
fn arc(a: Vec3, b: Vec3) -> f32 {
    a.dot(b).clamp(-1.0, 1.0).acos() * R
}

/// Avance de `d` unités le long du grand cercle qui part vers `f`.
fn walk(n: &mut Vec3, f: &mut Vec3, d: f32) {
    let q = Quat::from_axis_angle(n.cross(*f).normalize(), d / R);
    *n = (q * *n).normalize();
    *f = q * *f;
    *f = (*f - *n * f.dot(*n)).normalize(); // évite la dérive numérique
}

/// Tourne le regard de `a` radians autour de la verticale (négatif = droite).
fn turn(n: Vec3, f: &mut Vec3, a: f32) {
    *f = Quat::from_axis_angle(n, a) * *f;
}

/// Cercle de rayon `rad` (sur la surface) autour de `dir`, en directions unitaires.
fn circle(dir: Vec3, rad: f32) -> Vec<Vec3> {
    let (u, t) = (dir.any_orthonormal_vector(), rad / R);
    let v = dir.cross(u);
    (0..=32)
        .map(|i| {
            let a = i as f32 / 32.0 * TAU;
            dir * t.cos() + (u * a.cos() + v * a.sin()) * t.sin()
        })
        .collect()
}

// ---------- dessin ----------

fn ring(dir: Vec3, rad: f32, lift: f32, col: Color) {
    for w in circle(dir, rad).windows(2) {
        draw_line_3d(w[0] * (R + lift), w[1] * (R + lift), col);
    }
}

/// Disque posé sur la planète. Anneaux concentriques (0 = centre) pour épouser la courbure.
fn disc(dir: Vec3, rad: f32, lift: f32, col: Color) {
    let (mut vertices, mut indices) = (vec![], vec![]);
    for k in 0..=3 {
        for p in circle(dir, rad * k as f32 / 3.0) {
            vertices.push(Vertex::new2(p * (R + lift), Vec2::ZERO, col));
        }
    }
    for k in 0..3u16 {
        for i in 0..32u16 {
            let (a, c) = (k * 33 + i, (k + 1) * 33 + i);
            indices.extend([a, c, a + 1, a + 1, c, c + 1]);
        }
    }
    draw_mesh(&Mesh { vertices, indices, texture: None });
}

/// Boîte orientée (axes `ax`, demi-tailles `half`) : faces ombrées par le soleil + arêtes néon.
fn draw_box(c: Vec3, ax: [Vec3; 3], half: Vec3, col: Color) {
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
fn planet_grid() -> Vec<(Vec3, Vec3)> {
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

// ---------- le jeu ----------

/// Commandes d'un pas de simulation (clavier ou pilote automatique).
#[derive(Default)]
struct Input {
    fwd: bool,
    steer: f32, // > 0 : à gauche
    jump: bool,
}

/// Les touches sont physiques : sur AZERTY, W/A/D sont les touches Z/Q/D.
fn keyboard() -> Input {
    let key = |a, b| is_key_down(a) || is_key_down(b);
    Input {
        fwd: key(KeyCode::W, KeyCode::Up),
        steer: key(KeyCode::A, KeyCode::Left) as i32 as f32 - key(KeyCode::D, KeyCode::Right) as i32 as f32,
        jump: is_key_pressed(KeyCode::Space),
    }
}

struct Meteor {
    dir: Vec3,
    alt: f32,
    spin: f32,
}

struct Spark {
    pos: Vec3,
    vel: Vec3,
    life: f32,
    col: Color,
}

struct Game {
    n: Vec3, // « haut » du joueur (normale à la planète)
    f: Vec3, // direction du regard (tangente)
    h: f32,  // hauteur de saut
    vh: f32, // vitesse verticale
    trail: Vec<Vec3>,
    crystals: Vec<Vec3>,
    meteors: Vec<Meteor>,
    sparks: Vec<Spark>,
    score: u32,
    lives: u32,
    hurt: f32, // invulnérabilité restante
    shake: f32,
    next_meteor: f32,
    t: f32,
    cam: Vec3,
    cam_up: Vec3,
}

impl Game {
    fn new() -> Self {
        let mut g = Game {
            n: Vec3::Y,
            f: -Vec3::Z,
            h: 0.0,
            vh: 0.0,
            trail: vec![],
            crystals: vec![],
            meteors: vec![],
            sparks: vec![],
            score: 0,
            lives: 3,
            hurt: 0.0,
            shake: 0.0,
            next_meteor: 1.5,
            t: 0.0,
            cam: Vec3::ZERO,
            cam_up: Vec3::Y,
        };
        for _ in 0..3 {
            g.spawn_crystal();
        }
        g.cam = g.cam_goal();
        g
    }

    fn pos(&self) -> Vec3 {
        self.n * (R + self.h)
    }

    fn cam_goal(&self) -> Vec3 {
        // presque à la verticale : sur une petite sphère, c'est le seul angle qui montre le devant
        self.pos() + self.n * 10.0 - self.f * 3.5
    }

    fn spawn_crystal(&mut self) {
        loop {
            let d = rand_dir();
            if (5.0..14.0).contains(&arc(d, self.n)) {
                self.crystals.push(d);
                return;
            }
        }
    }

    fn burst(&mut self, pos: Vec3, col: Color, count: usize) {
        for _ in 0..count {
            let speed: f32 = rand::gen_range(0.4, 1.0);
            let vel = (rand_dir() * 5.0 + pos.normalize() * 4.0) * speed;
            self.sparks.push(Spark { pos, vel, life: rand::gen_range(0.4, 0.9), col });
        }
    }

    /// Ce qui avance même quand le joueur ne joue pas : horloge, étincelles.
    fn tick(&mut self, dt: f32) {
        self.t += dt;
        self.hurt = (self.hurt - dt).max(0.0);
        for s in &mut self.sparks {
            s.vel -= s.pos.normalize_or_zero() * 9.0 * dt; // gravité radiale
            s.pos += s.vel * dt;
            s.life -= dt;
        }
        self.sparks.retain(|s| s.life > 0.0);
    }

    /// Écran titre / fin : l'astronaute tourne doucement.
    fn idle(&mut self, dt: f32) {
        self.tick(dt);
        walk(&mut self.n, &mut self.f, 3.0 * dt);
        turn(self.n, &mut self.f, 0.4 * dt);
    }

    /// Pilote automatique : fonce vers le cristal le plus proche (tests et captures d'écran).
    fn bot(&self) -> Input {
        let near = self.crystals.iter().min_by(|a, b| arc(**a, self.n).total_cmp(&arc(**b, self.n)));
        let (fwd, steer) = near.map_or((true, 0.0), |&c| {
            let to = (c - self.n * c.dot(self.n)).normalize_or_zero(); // direction tangente vers c
            let (side, ahead) = (to.dot(self.f.cross(self.n)), to.dot(self.f)); // side > 0 : à droite
            let steer = if side.abs() < 0.05 && ahead > 0.0 {
                0.0
            } else if side > 0.0 {
                -1.0
            } else {
                1.0
            };
            (ahead > 0.3, steer) // n'avance que face au cristal, sinon il tourne en orbite autour
        });
        Input { fwd, steer, jump: false }
    }

    fn step(&mut self, dt: f32, inp: Input) {
        self.tick(dt);
        if inp.fwd {
            walk(&mut self.n, &mut self.f, 6.5 * dt);
        }
        turn(self.n, &mut self.f, inp.steer * 2.6 * dt);

        if inp.jump && self.h == 0.0 {
            self.vh = 8.0;
        }
        self.vh -= 22.0 * dt;
        self.h = (self.h + self.vh * dt).max(0.0);
        if self.h == 0.0 {
            self.vh = 0.0;
        }

        if self.trail.last().is_none_or(|&p| arc(p, self.n) > 0.4) {
            self.trail.push(self.n);
            if self.trail.len() > 45 {
                self.trail.remove(0);
            }
        }

        // cristaux
        let mut i = 0;
        while i < self.crystals.len() {
            let c = self.crystals[i];
            if arc(c, self.n) < PICKUP && self.h < 1.8 {
                self.crystals.swap_remove(i);
                self.score += 1;
                self.burst(c * (R + 1.0), CYAN, 18);
                self.spawn_crystal();
            } else {
                i += 1;
            }
        }

        // météorites : de plus en plus fréquentes
        self.next_meteor -= dt;
        if self.next_meteor <= 0.0 {
            self.next_meteor = (2.0 - 0.07 * self.score as f32).max(0.55);
            let dir = (self.n + rand_dir() * 0.7).normalize();
            self.meteors.push(Meteor { dir, alt: ALT0, spin: rand::gen_range(0.0, TAU) });
        }
        let mut i = 0;
        while i < self.meteors.len() {
            self.meteors[i].alt -= FALL * dt;
            if self.meteors[i].alt > 0.0 {
                i += 1;
                continue;
            }
            let m = self.meteors.swap_remove(i);
            self.burst(m.dir * R, FIRE, 30);
            self.shake = self.shake.max(0.4);
            if arc(m.dir, self.n) < BLAST && self.h < 0.8 && self.hurt == 0.0 {
                self.lives -= 1;
                self.hurt = 1.5;
                self.shake = 1.0;
            }
        }
    }

    /// Caméra à la troisième personne, lissée, avec « haut » = verticale du joueur.
    fn camera(&mut self, dt: f32) -> Camera3D {
        let k = 1.0 - (-6.0 * dt).exp();
        self.cam = self.cam.lerp(self.cam_goal(), k);
        self.cam_up = self.cam_up.lerp(self.n, k).normalize();
        self.shake = (self.shake - 2.0 * dt).max(0.0);
        let jitter = vec3(rand::gen_range(-1.0, 1.0), rand::gen_range(-1.0, 1.0), rand::gen_range(-1.0, 1.0));
        Camera3D {
            position: self.cam + jitter * self.shake * 0.4,
            target: self.pos() + self.f * 2.0,
            up: self.cam_up,
            fovy: 1.0,
            z_near: 0.5,
            z_far: 1000.0,
            ..Default::default()
        }
    }

    fn draw(&self, stars: &[Vec3], grid: &[(Vec3, Vec3)]) {
        // ciel
        for &s in stars {
            draw_cube(s, Vec3::splat(1.2), None, WHITE);
        }
        let smooth = |rings, slices| DrawSphereParams { rings, slices, ..Default::default() };
        let sun = SUN.normalize() * 260.0;
        draw_sphere_ex(sun, 22.0, None, Color::new(1.0, 0.62, 0.2, 1.0), smooth(24, 48));
        draw_sphere_ex(sun, 34.0, None, Color::new(1.0, 0.4, 0.3, 0.18), smooth(24, 48)); // halo, dessiné après le disque

        // planète
        draw_sphere_ex(Vec3::ZERO, R, None, Color::new(0.12, 0.06, 0.26, 1.0), smooth(40, 64));
        for &(a, b) in grid {
            draw_line_3d(a, b, Color::new(1.0, 0.2, 0.7, 0.55));
        }

        // marquages au sol : aura du joueur, zones d'impact, traînée
        disc(self.n, PICKUP, 0.05, Color { a: 0.06, ..CYAN });
        ring(self.n, PICKUP, 0.06, Color { a: 0.5, ..CYAN });
        for m in &self.meteors {
            let col = if arc(m.dir, self.n) < BLAST { Color::new(1.0, 0.15, 0.1, 1.0) } else { FIRE };
            disc(m.dir, BLAST, 0.03, Color { a: 0.18, ..col });
            ring(m.dir, BLAST, 0.06, col);
            ring(m.dir, BLAST * (1.0 - m.alt / ALT0), 0.06, col); // se remplit jusqu'à l'impact
            draw_line_3d(m.dir * R, m.dir * (R + m.alt), Color { a: 0.3, ..col }); // trajectoire de chute
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

        // cristaux (avec un faisceau visible depuis l'autre côté de l'horizon)
        for (i, &c) in self.crystals.iter().enumerate() {
            let bob = (self.t * 3.0 + i as f32 * 2.0).sin() * 0.15;
            let q = Quat::from_axis_angle(c, self.t * 2.0 + i as f32);
            let u = c.any_orthonormal_vector();
            draw_line_3d(c * R, c * (R + 12.0), Color { a: 0.35, ..CYAN });
            draw_box(c * (R + 1.7 + bob), [q * u, c, q * c.cross(u)], vec3(0.36, 0.8, 0.36), CYAN);
        }

        // météorites
        for m in &self.meteors {
            let p = m.dir * (R + m.alt);
            let q = Quat::from_euler(EulerRot::XYZ, self.t * 2.1 + m.spin, self.t * 1.3 + m.spin * 2.0, m.spin);
            draw_line_3d(p, p + m.dir * 6.0, FIRE);
            draw_box(p, [q * Vec3::X, q * Vec3::Y, q * Vec3::Z], Vec3::splat(1.2), FIRE);
        }

        // astronaute (clignote quand il est invulnérable)
        if self.hurt == 0.0 || (self.t * 14.0) as i32 % 2 == 0 {
            let (n, f, p) = (self.n, self.f, self.pos());
            let ax = [f.cross(n), n, f];
            let (suit, s) = (Color::new(0.85, 0.9, 1.0, 1.0), 1.3); // s : échelle de l'astronaute
            draw_box(p + n * 0.45 * s, ax, vec3(0.28, 0.4, 0.2) * s, suit);
            draw_box(p + n * 1.05 * s, ax, vec3(0.2, 0.18, 0.2) * s, suit);
            draw_box(p + n * 1.07 * s + f * 0.17 * s, ax, vec3(0.15, 0.07, 0.06) * s, CYAN);
            draw_box(p + n * 0.55 * s - f * 0.28 * s, ax, vec3(0.17, 0.26, 0.1) * s, PINK);
        }

        for s in &self.sparks {
            draw_cube(s.pos, Vec3::splat(0.1 + 0.15 * s.life), None, Color { a: (s.life * 2.0).min(1.0), ..s.col });
        }
    }
}

// ---------- boucle principale ----------

#[derive(PartialEq)]
enum State {
    Title,
    Play,
    Over,
}

fn say(s: &str, y: f32, size: f32, col: Color) {
    let w = measure_text(s, None, size as u16, 1.0).width;
    draw_text(s, (screen_width() - w) / 2.0, y, size, col);
}

fn conf() -> Conf {
    Conf {
        window_title: "Petite Planète".into(),
        window_width: 1280,
        window_height: 720,
        sample_count: 4,
        ..Default::default()
    }
}

#[macroquad::main(conf)]
async fn main() {
    rand::srand(macroquad::miniquad::date::now() as u64);
    // Outil de test : `petite-planete play|title [images]` (240 par défaut) joue en pilote
    // automatique, enregistre shot-<mode>-<images>.png puis quitte.
    let mut args = std::env::args().skip(1);
    let shot = args.next();
    let shot_at: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(240);
    let auto = shot.as_deref() == Some("play");
    let stars: Vec<Vec3> = (0..300).map(|_| rand_dir() * 300.0).collect();
    let grid = planet_grid();
    let mut g = Game::new();
    let mut state = if auto { State::Play } else { State::Title };
    if state == State::Title {
        g.crystals.clear(); // écran titre : seulement la planète et l'astronaute
    }
    let (mut best, mut frames) = (0, 0);

    loop {
        let dt = get_frame_time().min(0.05);
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        match state {
            State::Play => {
                let inp = if auto { g.bot() } else { keyboard() };
                g.step(dt, inp);
                if g.lives == 0 {
                    best = best.max(g.score);
                    g.meteors.clear();
                    state = State::Over;
                }
            }
            _ => {
                g.idle(dt);
                if is_key_pressed(KeyCode::Space) {
                    g = Game::new();
                    state = State::Play;
                }
            }
        }

        clear_background(Color::new(0.04, 0.03, 0.12, 1.0));
        set_camera(&g.camera(dt));
        g.draw(&stars, &grid);

        set_default_camera();
        if g.hurt > 0.0 {
            let (w, h) = (screen_width(), screen_height());
            draw_rectangle(0.0, 0.0, w, h, Color::new(1.0, 0.1, 0.1, g.hurt.min(1.0) * 0.25));
        }
        if state != State::Title {
            draw_text(format!("CRISTAUX  {}", g.score), 24.0, 44.0, 40.0, WHITE);
            for i in 0..g.lives {
                draw_rectangle(24.0 + i as f32 * 30.0, 58.0, 22.0, 22.0, PINK);
            }
            draw_text(format!("RECORD  {best}"), screen_width() - 200.0, 44.0, 32.0, GRAY);
        }
        match state {
            State::Title => {
                say("PETITE PLANÈTE", 200.0, 96.0, CYAN);
                say("Ramasse les cristaux, évite les météorites", 260.0, 34.0, WHITE);
                say("Flèches ou ZQSD/WASD : bouger     ESPACE : sauter", 310.0, 28.0, GRAY);
                say("Appuie sur ESPACE", 670.0, 36.0, PINK);
            }
            State::Over => {
                say("PERDU !", 280.0, 96.0, PINK);
                say(&format!("{} cristaux", g.score), 340.0, 40.0, WHITE);
                say("ESPACE pour rejouer", 420.0, 36.0, CYAN);
            }
            State::Play => {}
        }

        if let Some(mode) = &shot {
            frames += 1;
            if frames == shot_at {
                get_screen_data().export_png(&format!("shot-{mode}-{shot_at}.png"));
                break;
            }
        }
        next_frame().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walking_stays_on_the_sphere() {
        let (mut n, mut f) = (Vec3::Y, -Vec3::Z);
        for i in 0..2000 {
            walk(&mut n, &mut f, 0.37);
            turn(n, &mut f, if i % 7 == 0 { 0.4 } else { -0.05 });
        }
        assert!((n.length() - 1.0).abs() < 1e-4);
        assert!(n.dot(f).abs() < 1e-3);
    }

    #[test]
    fn walking_covers_the_given_distance() {
        let (mut n, mut f) = (Vec3::Y, -Vec3::Z);
        walk(&mut n, &mut f, 1.0);
        assert!((arc(Vec3::Y, n) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn turning_right_moves_the_gaze_to_the_right() {
        let (n, mut f) = (Vec3::Y, -Vec3::Z);
        let right = f.cross(n);
        turn(n, &mut f, -0.1);
        assert!(f.dot(right) > 0.0);
    }

    #[test]
    fn bot_collects_crystals() {
        let mut g = Game::new();
        for _ in 0..60 * 30 {
            let inp = g.bot();
            g.step(1.0 / 60.0, inp);
            if g.lives == 0 {
                break; // le bot ne dodge pas : il peut mourir
            }
        }
        assert!(g.score >= 5, "score = {}", g.score);
        assert_eq!(g.crystals.len(), 3); // chaque cristal ramassé est remplacé
    }

    #[test]
    fn meteor_hurts_on_the_ground_but_not_in_the_air() {
        let mut g = Game::new();
        g.meteors.push(Meteor { dir: g.n, alt: 0.01, spin: 0.0 });
        g.step(0.1, Input::default());
        assert_eq!(g.lives, 2);
        assert!(g.meteors.is_empty());

        let mut g = Game::new();
        g.h = 2.0;
        g.meteors.push(Meteor { dir: g.n, alt: 0.01, spin: 0.0 });
        g.step(0.1, Input::default());
        assert_eq!(g.lives, 3);
    }
}
