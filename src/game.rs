//! Simulation du jeu : aucune dépendance au dessin, donc testable sans fenêtre.
use crate::sphere::*;
use macroquad::prelude::*;
use std::f32::consts::TAU;

// ---------- réglages : tout ce qui change la difficulté est ici ----------

pub const BLAST: f32 = 3.0; // rayon de souffle d'une météorite
pub const ALT0: f32 = 26.0; // altitude de départ des météorites
pub const FALL: f32 = 12.0; // vitesse de chute des météorites
pub const PICKUP: f32 = 1.6; // rayon de ramassage d'un cristal
pub const SHOCK_MAX: f32 = 10.0; // rayon maximal d'une onde d'impact
pub const DASH_TIME: f32 = 0.2; // durée du dash
pub const DASH_CD: f32 = 1.4; // recharge du dash
pub const CHAIN_TIME: f32 = 3.0; // temps pour enchaîner un ramassage sans casser le combo
pub const MAX_LIVES: u32 = 5;
const WALK: f32 = 6.5; // vitesse de marche (unités/s)
const DASH_SPEED: f32 = 24.0;
const TURN: f32 = 2.6; // vitesse de rotation (rad/s)
const CRYSTAL_LIFE: f32 = 11.0; // durée de vie d'un cristal (s)
const GOLD_LIFE: f32 = 7.0;
const SHOCK_SPEED: f32 = 7.0;
const REST: f32 = 3.5; // pause entre deux vagues (s)

pub const CYAN: Color = Color::new(0.15, 0.95, 1.0, 1.0);
pub const HOTPINK: Color = Color::new(1.0, 0.2, 0.7, 1.0);
pub const FIRE: Color = Color::new(1.0, 0.5, 0.1, 1.0);
pub const AMBER: Color = Color::new(1.0, 0.82, 0.2, 1.0);
pub const DRONE: Color = Color::new(1.0, 0.15, 0.35, 1.0);

/// Réglages d'une vague : chacune ajoute un danger aux précédents.
pub struct Cfg {
    pub quota: u32,   // cristaux à ramasser pour finir la vague
    pub every: f32,   // intervalle moyen entre deux météorites (s)
    pub shocks: bool, // les impacts lancent des ondes
    pub drones: usize, // drones poursuivants
    pub salvos: bool, // salves de météorites en anneau
    pub lead: f32,    // part des météorites qui visent devant le joueur
}

pub fn cfg(w: u32) -> Cfg {
    Cfg {
        quota: 6 + 2 * w,
        every: (2.2 - 0.2 * w as f32).max(0.7),
        shocks: w >= 2,
        drones: if w >= 3 { ((w - 1) / 2).min(4) as usize } else { 0 },
        salvos: w >= 4,
        lead: (0.15 * (w as f32 - 1.0)).min(0.5),
    }
}

/// Générateur pseudo-aléatoire (xorshift64*) propre à la partie : les simulations sont reproductibles.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    /// Flottant uniforme dans [lo, hi).
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let r = self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40; // 24 bits
        lo + (hi - lo) * r as f32 / (1u64 << 24) as f32
    }

    /// Direction unitaire uniforme sur la sphère.
    pub fn dir(&mut self) -> Vec3 {
        let z = self.range(-1.0, 1.0);
        let a = self.range(0.0, TAU);
        let r = (1.0 - z * z).sqrt();
        vec3(r * a.cos(), z, r * a.sin())
    }
}

/// Commandes d'un pas de simulation (clavier ou pilote automatique).
#[derive(Default)]
pub struct Input {
    pub fwd: bool,
    pub steer: f32, // > 0 : à gauche
    pub jump: bool,
    pub dash: bool,
}

pub struct Crystal {
    pub dir: Vec3,
    pub life: f32,
    pub gold: bool, // cristal doré : posé au centre d'une zone d'impact
}

pub struct Meteor {
    pub dir: Vec3,
    pub alt: f32,
    pub spin: f32,
}

/// Onde circulaire qui part d'un impact (`harmful`) ou célèbre une vague terminée.
pub struct Shock {
    pub dir: Vec3,
    pub r: f32,
    pub harmful: bool,
}

/// Drone poursuivant : se déplace sur la sphère comme le joueur.
pub struct Drone {
    pub n: Vec3,
    pub f: Vec3,
    pub arm: f32, // temps restant avant qu'il devienne actif
}

pub struct Spark {
    pub pos: Vec3,
    pub vel: Vec3,
    pub life: f32,
    pub col: Color,
}

/// Texte flottant (points gagnés), affiché par l'interface au-dessus de `pos`.
pub struct Popup {
    pub pos: Vec3,
    pub text: String,
    pub life: f32,
    pub col: Color,
}

pub struct Game {
    pub rng: Rng,
    // joueur
    pub n: Vec3, // « haut » du joueur (normale à la planète)
    pub f: Vec3, // direction du regard (tangente)
    pub h: f32,  // hauteur de saut
    pub vh: f32, // vitesse verticale
    pub moving: bool,
    pub stride: f32, // distance parcourue au sol : anime les jambes
    pub jump_buf: f32, // saut demandé juste avant l'atterrissage : il part dès qu'on touche le sol
    pub dash_t: f32,  // temps de dash restant
    pub dash_cd: f32, // recharge restante
    pub trail: Vec<Vec3>,
    // monde
    pub crystals: Vec<Crystal>,
    pub meteors: Vec<Meteor>,
    pub shocks: Vec<Shock>,
    pub drones: Vec<Drone>,
    pub sparks: Vec<Spark>,
    pub popups: Vec<Popup>,
    // partie
    pub score: u32,
    pub lives: u32,
    pub kills: u32,
    pub wave: u32,
    pub got: u32,       // progression du quota de la vague
    pub rest: f32,      // pause sans danger restante
    pub banner: f32,    // temps restant d'affichage de la bannière de vague
    pub flawless: bool, // aucun coup reçu depuis le début de la vague
    pub chain: u32,
    pub chain_t: f32,
    pub hurt: f32, // invulnérabilité restante après un coup
    pub shake: f32,
    pub next_meteor: f32,
    pub next_salvo: f32,
    pub drone_cd: f32,
    pub t: f32,
    pub cam: Vec3,
    pub cam_up: Vec3,
}

/// Pour aller de (n, f) vers `target` : (virage −1/0/+1 avec + = gauche, cosinus de l'écart de cap).
fn aim(n: Vec3, f: Vec3, target: Vec3) -> (f32, f32) {
    let to = (target - n * target.dot(n)).normalize_or_zero(); // direction tangente vers la cible
    let (side, ahead) = (to.dot(f.cross(n)), to.dot(f)); // side > 0 : la cible est à droite
    let steer = if side.abs() < 0.05 && ahead > 0.0 {
        0.0
    } else if side > 0.0 {
        -1.0
    } else {
        1.0
    };
    (steer, ahead)
}

impl Game {
    pub fn new(seed: u64) -> Self {
        let mut g = Game {
            rng: Rng::new(seed),
            n: Vec3::Y,
            f: -Vec3::Z,
            h: 0.0,
            vh: 0.0,
            moving: false,
            stride: 0.0,
            jump_buf: 0.0,
            dash_t: 0.0,
            dash_cd: 0.0,
            trail: vec![],
            crystals: vec![],
            meteors: vec![],
            shocks: vec![],
            drones: vec![],
            sparks: vec![],
            popups: vec![],
            score: 0,
            lives: 3,
            kills: 0,
            wave: 0,
            got: 0,
            rest: 0.0,
            banner: 0.0,
            flawless: true,
            chain: 0,
            chain_t: 0.0,
            hurt: 0.0,
            shake: 0.0,
            next_meteor: 0.0,
            next_salvo: 0.0,
            drone_cd: 0.0,
            t: 0.0,
            cam: Vec3::ZERO,
            cam_up: Vec3::Y,
        };
        g.start_wave(1);
        g.cam = g.cam_goal();
        g
    }

    pub fn pos(&self) -> Vec3 {
        self.n * (R + self.h)
    }

    pub fn cam_goal(&self) -> Vec3 {
        // presque à la verticale : sur une petite sphère, c'est le seul angle qui montre le devant
        self.pos() + self.n * 10.0 - self.f * 3.5
    }

    /// Multiplicateur de combo, de 1 à 5.
    pub fn mult(&self) -> u32 {
        self.chain.clamp(1, 5)
    }

    /// Démarre la vague `w` : pause sans danger, bannière, minuteurs remis à zéro.
    pub fn start_wave(&mut self, w: u32) {
        self.wave = w;
        self.got = 0;
        self.flawless = true;
        self.rest = if w == 1 { 1.5 } else { REST };
        self.banner = 4.0;
        self.next_meteor = 0.5;
        self.next_salvo = self.rng.range(10.0, 16.0);
        self.drone_cd = 1.0;
        self.fill_crystals();
    }

    /// Garde toujours 3 cristaux ordinaires, jamais dans une zone d'impact ni sous le joueur.
    fn fill_crystals(&mut self) {
        while self.crystals.iter().filter(|c| !c.gold).count() < 3 {
            let d = self.rng.dir();
            if (5.0..14.0).contains(&arc(d, self.n)) && !self.meteors.iter().any(|m| arc(m.dir, d) < BLAST) {
                self.crystals.push(Crystal { dir: d, life: CRYSTAL_LIFE, gold: false });
            }
        }
    }

    pub fn burst(&mut self, pos: Vec3, col: Color, count: usize) {
        for _ in 0..count {
            let speed: f32 = self.rng.range(0.4, 1.0);
            let vel = (self.rng.dir() * 5.0 + pos.normalize() * 4.0) * speed;
            self.sparks.push(Spark { pos, vel, life: self.rng.range(0.4, 0.9), col });
        }
    }

    fn pop(&mut self, pos: Vec3, text: impl Into<String>, col: Color) {
        self.popups.push(Popup { pos, text: text.into(), life: 1.2, col });
    }

    fn vulnerable(&self) -> bool {
        self.hurt == 0.0 && self.dash_t == 0.0
    }

    /// Un coup : une vie en moins, 1,5 s d'invulnérabilité, combo et « sans faute » perdus.
    fn hit(&mut self) {
        self.lives = self.lives.saturating_sub(1);
        self.hurt = 1.5;
        self.shake = 1.0;
        self.chain = 0;
        self.chain_t = 0.0;
        self.flawless = false;
    }

    /// Un maillon de plus dans la chaîne (cristal ou drone écrasé) : renvoie le multiplicateur.
    fn chain_up(&mut self) -> u32 {
        self.chain = if self.chain_t > 0.0 { self.chain + 1 } else { 1 };
        self.chain_t = CHAIN_TIME;
        self.mult()
    }

    /// Supprime tous les dangers en cours (fin de vague, fin de partie).
    pub fn clear_hazards(&mut self) {
        for m in std::mem::take(&mut self.meteors) {
            self.burst(m.dir * (R + m.alt), FIRE, 10);
        }
        for d in std::mem::take(&mut self.drones) {
            self.burst(d.n * (R + 0.6), DRONE, 12);
        }
        self.shocks.clear();
    }

    fn clear_wave(&mut self) {
        let bonus = 100 * self.wave * if self.flawless { 2 } else { 1 };
        self.score += bonus;
        self.lives = (self.lives + 1).min(MAX_LIVES);
        self.clear_hazards();
        self.shocks.push(Shock { dir: self.n, r: 0.0, harmful: false });
        self.pop(self.pos() + self.n * 2.0, format!("VAGUE TERMINÉE +{bonus}"), CYAN);
        self.start_wave(self.wave + 1);
    }

    /// Ce qui avance même quand le joueur ne joue pas : horloge, minuteurs, étincelles.
    pub fn tick(&mut self, dt: f32) {
        self.t += dt;
        let timers = [&mut self.hurt, &mut self.dash_t, &mut self.dash_cd, &mut self.rest];
        for timer in timers.into_iter().chain([&mut self.banner, &mut self.chain_t, &mut self.jump_buf]) {
            *timer = (*timer - dt).max(0.0);
        }
        if self.chain_t == 0.0 {
            self.chain = 0;
        }
        for s in &mut self.sparks {
            s.vel -= s.pos.normalize_or_zero() * 9.0 * dt; // gravité radiale
            s.pos += s.vel * dt;
            s.life -= dt;
        }
        self.sparks.retain(|s| s.life > 0.0);
        for p in &mut self.popups {
            p.pos += p.pos.normalize_or_zero() * 1.5 * dt;
            p.life -= dt;
        }
        self.popups.retain(|p| p.life > 0.0);
    }

    /// Écran titre / fin : l'astronaute tourne doucement.
    pub fn idle(&mut self, dt: f32) {
        self.tick(dt);
        walk(&mut self.n, &mut self.f, 3.0 * dt);
        turn(self.n, &mut self.f, 0.4 * dt);
    }

    /// Pilote automatique (tests et captures d'écran) : fuit les impacts, saute les ondes et les
    /// drones, dash en dernier recours, ramasse le cristal le plus proche qui ne sera pas détruit.
    pub fn bot(&self) -> Input {
        let here = |d: Vec3| arc(d, self.n);
        let threat = self
            .meteors
            .iter()
            .filter(|m| m.alt / FALL < 2.2 && here(m.dir) < BLAST + 1.0)
            .min_by(|a, b| a.alt.total_cmp(&b.alt));
        let doomed = |c: &Crystal| {
            self.meteors
                .iter()
                .any(|m| arc(m.dir, c.dir) < BLAST && (!c.gold || here(c.dir) / WALK > m.alt / FALL - 0.3))
        };
        let (goal, fleeing) = match threat {
            Some(m) => {
                let away = (self.n - m.dir * self.n.dot(m.dir)).normalize_or_zero();
                (Some((self.n + away * 0.5).normalize()), true)
            }
            None => {
                let near = self.crystals.iter().filter(|c| !doomed(c)).min_by(|a, b| here(a.dir).total_cmp(&here(b.dir)));
                (near.map(|c| c.dir), false)
            }
        };
        let (steer, ahead) = goal.map_or((0.0, 0.0), |g| aim(self.n, self.f, g));
        let wave_coming = self.shocks.iter().any(|s| s.harmful && (0.0..1.6).contains(&(here(s.dir) - s.r)));
        let drone_close = self.drones.iter().any(|d| d.arm <= 0.0 && here(d.n) < 2.2);
        Input {
            fwd: goal.is_some() && (fleeing || ahead > 0.3), // n'avance que face au but, sinon il tourne en orbite
            steer,
            jump: self.h == 0.0 && (wave_coming || drone_close),
            dash: threat.is_some_and(|m| m.alt / FALL < 0.8 && here(m.dir) < BLAST),
        }
    }

    fn spawn_meteor(&mut self, lead: f32) {
        let mut aim_at = self.n;
        if self.moving && self.rng.range(0.0, 1.0) < lead {
            let mut f = self.f;
            walk(&mut aim_at, &mut f, WALK * ALT0 / FALL); // là où sera le joueur à l'impact
        }
        let dir = (aim_at + self.rng.dir() * 0.7).normalize();
        self.meteors.push(Meteor { dir, alt: ALT0, spin: self.rng.range(0.0, TAU) });
        if self.wave >= 2 && self.rng.range(0.0, 1.0) < 0.25 {
            self.crystals.push(Crystal { dir, life: GOLD_LIFE, gold: true }); // appât au centre de la zone
        }
    }

    /// Anneau de 6 météorites autour du joueur, impacts échelonnés : le centre est sûr.
    fn salvo(&mut self) {
        let ring = circle(self.n, 7.0);
        let phase = self.rng.range(0.0, 32.0) as usize;
        for i in 0..6 {
            let dir = ring[(i * 5 + phase) % 32];
            self.meteors.push(Meteor { dir, alt: ALT0 + 1.5 * i as f32, spin: self.rng.range(0.0, TAU) });
        }
    }

    fn spawn_drone(&mut self) {
        loop {
            let n = self.rng.dir();
            if (12.0..22.0).contains(&arc(n, self.n)) {
                self.drones.push(Drone { n, f: n.any_orthonormal_vector(), arm: 1.2 });
                self.drone_cd = 4.0;
                return;
            }
        }
    }

    /// Explosion en `dir` : onde, destruction des cristaux et des drones dans le rayon, coup au joueur.
    fn impact(&mut self, dir: Vec3, shock: bool) {
        self.burst(dir * R, FIRE, 30);
        self.shake = self.shake.max(0.4);
        if shock {
            self.shocks.push(Shock { dir, r: BLAST, harmful: true });
        }
        let mut lost = vec![];
        self.crystals.retain(|c| {
            let hit = arc(c.dir, dir) < BLAST;
            if hit {
                lost.push(c.dir);
            }
            !hit
        });
        for d in lost {
            self.burst(d * (R + 1.0), GRAY, 6);
        }
        let mut crushed = vec![];
        self.drones.retain(|d| {
            let hit = arc(d.n, dir) < BLAST;
            if hit {
                crushed.push(d.n);
            }
            !hit
        });
        for p in crushed {
            self.kills += 1;
            self.score += 20;
            self.drone_cd = 4.0;
            self.burst(p * (R + 0.6), DRONE, 16);
            self.pop(p * (R + 1.5), "+20", DRONE);
        }
        if arc(dir, self.n) < BLAST && self.h < 0.8 && self.vulnerable() {
            self.hit();
        }
    }

    /// Le joueur retombe sur un drone : rebond, points, maillon de combo.
    fn stomp(&mut self, p: Vec3) {
        self.vh = 8.0;
        self.kills += 1;
        self.drone_cd = 4.0;
        let pts = 30 * self.chain_up();
        self.score += pts;
        self.burst(p * (R + 0.6), DRONE, 20);
        self.pop(p * (R + 1.5), format!("+{pts}"), DRONE);
    }

    pub fn step(&mut self, dt: f32, inp: Input) {
        self.tick(dt);
        let c = cfg(self.wave);

        // déplacement : marche ou dash (invulnérable)
        if inp.dash && self.dash_cd == 0.0 {
            self.dash_t = DASH_TIME;
            self.dash_cd = DASH_CD;
            self.burst(self.pos(), CYAN, 12);
        }
        let dashing = self.dash_t > 0.0;
        self.moving = inp.fwd || dashing;
        if self.moving {
            let speed = if dashing { DASH_SPEED } else { WALK };
            walk(&mut self.n, &mut self.f, speed * dt);
            if self.h == 0.0 {
                self.stride += speed * dt;
            }
        }
        turn(self.n, &mut self.f, inp.steer * TURN * dt);

        // saut (mémorisé 0,12 s : demandé un peu avant l'atterrissage, il part quand même)
        if inp.jump {
            self.jump_buf = 0.12;
        }
        if self.jump_buf > 0.0 && self.h == 0.0 {
            self.vh = 8.0;
            self.jump_buf = 0.0;
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

        // cristaux : ils expirent, on les ramasse en chaîne
        for k in &mut self.crystals {
            k.life -= dt;
        }
        self.crystals.retain(|k| k.life > 0.0);
        let mut i = 0;
        while i < self.crystals.len() {
            if arc(self.crystals[i].dir, self.n) < PICKUP && self.h < 1.8 {
                let k = self.crystals.swap_remove(i);
                let (base, worth, col) = if k.gold { (50, 3, AMBER) } else { (10, 1, CYAN) };
                let pts = base * self.chain_up();
                self.score += pts;
                self.got += worth;
                self.burst(k.dir * (R + 1.0), col, if k.gold { 30 } else { 18 });
                self.pop(k.dir * (R + 2.0), format!("+{pts}"), col);
            } else {
                i += 1;
            }
        }
        self.fill_crystals();
        if self.got >= c.quota {
            self.clear_wave();
            return;
        }

        // dangers : rien ne tombe pendant la pause entre deux vagues
        if self.rest == 0.0 {
            self.next_meteor -= dt;
            if self.next_meteor <= 0.0 {
                self.next_meteor = c.every * self.rng.range(0.7, 1.3);
                self.spawn_meteor(c.lead);
            }
            if c.salvos {
                self.next_salvo -= dt;
                if self.next_salvo <= 0.0 {
                    self.next_salvo = self.rng.range(10.0, 16.0);
                    self.salvo();
                }
            }
            self.drone_cd = (self.drone_cd - dt).max(0.0);
            if self.drones.len() < c.drones && self.drone_cd == 0.0 {
                self.spawn_drone();
            }
        }

        // ondes : touchent si on est au sol quand l'anneau passe
        for s in &mut self.shocks {
            s.r += SHOCK_SPEED * dt;
        }
        let n = self.n;
        if self.h < 0.5 && self.vulnerable() && self.shocks.iter().any(|s| s.harmful && (arc(s.dir, n) - s.r).abs() < 0.6) {
            self.hit();
        }
        self.shocks.retain(|s| s.r < if s.harmful { SHOCK_MAX } else { 2.0 * SHOCK_MAX });

        // météorites
        let mut i = 0;
        while i < self.meteors.len() {
            self.meteors[i].alt -= FALL * dt;
            if self.meteors[i].alt > 0.0 {
                i += 1;
                continue;
            }
            let dir = self.meteors.swap_remove(i).dir;
            self.impact(dir, c.shocks);
        }

        // drones : poursuite, écrasement (on retombe dessus), contact (coup)
        let (pn, ph, pvh) = (self.n, self.h, self.vh);
        let speed = (2.6 + 0.3 * self.wave as f32).min(4.6);
        let (mut stomped, mut touched) = (vec![], false);
        self.drones.retain_mut(|d| {
            if d.arm > 0.0 {
                d.arm -= dt;
            } else {
                let (steer, _) = aim(d.n, d.f, pn);
                turn(d.n, &mut d.f, steer * 2.4 * dt);
                walk(&mut d.n, &mut d.f, speed * dt);
            }
            let a = arc(d.n, pn);
            if ph > 0.35 && pvh < 0.0 && a < 1.5 {
                stomped.push(d.n);
                return false;
            }
            touched |= d.arm <= 0.0 && a < 0.9 && ph < 1.0;
            true
        });
        for p in stomped {
            self.stomp(p);
        }
        if touched && self.vulnerable() {
            self.hit();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    /// Partie sans danger : la pause entre deux vagues ne finit jamais.
    fn calm() -> Game {
        let mut g = Game::new(1);
        g.rest = 1e6;
        g
    }

    fn crystal_here(g: &mut Game, gold: bool) {
        g.crystals.push(Crystal { dir: g.n, life: 5.0, gold });
    }

    fn meteor_here(g: &mut Game) {
        g.meteors.push(Meteor { dir: g.n, alt: 0.01, spin: 0.0 });
    }

    #[test]
    fn combo_chains_within_the_window_and_resets_after() {
        let mut g = calm();
        crystal_here(&mut g, false);
        g.step(DT, Input::default());
        assert_eq!((g.score, g.mult()), (10, 1));
        crystal_here(&mut g, false);
        g.step(DT, Input::default());
        assert_eq!((g.score, g.mult()), (10 + 20, 2));
        for _ in 0..(CHAIN_TIME * 60.0) as usize + 5 {
            g.step(DT, Input::default());
        }
        assert_eq!(g.chain, 0);
        crystal_here(&mut g, false);
        g.step(DT, Input::default());
        assert_eq!(g.score, 10 + 20 + 10);
    }

    #[test]
    fn a_jump_pressed_just_before_landing_is_not_lost() {
        let mut g = calm();
        g.h = 0.05;
        g.vh = -5.0;
        g.step(DT, Input { jump: true, ..Default::default() }); // encore en l'air : touche le sol pendant ce pas
        for _ in 0..3 {
            g.step(DT, Input::default());
        }
        assert!(g.h > 0.0, "le saut mémorisé doit partir à l'atterrissage");
    }

    #[test]
    fn stride_counts_the_distance_walked_on_the_ground_only() {
        let mut g = calm();
        for _ in 0..60 {
            g.step(DT, Input { fwd: true, ..Default::default() });
        }
        assert!((g.stride - WALK).abs() < 0.2, "foulée {}", g.stride); // 1 s à 6,5 u/s
        let s = g.stride;
        g.h = 2.0;
        g.step(DT, Input { fwd: true, ..Default::default() });
        assert_eq!(g.stride, s, "en l'air, les jambes ne comptent pas");
        let mut still = calm();
        still.step(DT, Input::default());
        assert_eq!(still.stride, 0.0);
    }

    #[test]
    fn crystals_expire_and_are_replaced() {
        let mut g = calm();
        g.crystals[0].life = 0.005;
        g.step(DT, Input::default());
        assert_eq!(g.crystals.len(), 3);
        assert!(g.crystals.iter().all(|c| c.life > 0.0));
        assert_eq!(g.score, 0);
    }

    #[test]
    fn gold_crystal_is_worth_more_and_counts_triple() {
        let mut g = calm();
        crystal_here(&mut g, true);
        g.step(DT, Input::default());
        assert_eq!((g.score, g.got), (50, 3));
    }

    #[test]
    fn finishing_a_wave_pays_a_bonus_heals_and_clears_hazards() {
        let mut g = calm();
        g.got = cfg(1).quota - 1;
        g.meteors.push(Meteor { dir: Vec3::X, alt: 20.0, spin: 0.0 });
        g.drones.push(Drone { n: Vec3::X, f: Vec3::Y, arm: 0.0 });
        crystal_here(&mut g, false);
        g.step(DT, Input::default());
        assert_eq!((g.wave, g.got, g.lives), (2, 0, 4));
        assert_eq!(g.score, 10 + 200); // cristal + bonus de vague doublé (sans faute)
        assert!(g.meteors.is_empty() && g.drones.is_empty());
        assert!(g.rest > 3.0 && g.banner > 0.0);

        // avec un coup reçu : bonus simple, et la vie est plafonnée
        let mut g = calm();
        g.lives = MAX_LIVES;
        g.flawless = false;
        g.got = cfg(1).quota - 1;
        crystal_here(&mut g, false);
        g.step(DT, Input::default());
        assert_eq!((g.score, g.lives), (10 + 100, MAX_LIVES));
    }

    #[test]
    fn no_hazard_spawns_during_the_rest() {
        let mut g = Game::new(1);
        for _ in 0..60 {
            g.step(DT, Input::default());
        }
        assert!(g.rest > 0.0 && g.meteors.is_empty());
        for _ in 0..60 * 3 {
            g.step(DT, Input::default());
        }
        assert!(g.rest == 0.0 && !g.meteors.is_empty() || g.lives < 3);
    }

    #[test]
    fn meteor_hurts_on_the_ground_but_not_in_the_air() {
        let mut g = calm();
        meteor_here(&mut g);
        g.step(0.1, Input::default());
        assert_eq!(g.lives, 2);
        assert!(g.meteors.is_empty());

        let mut g = calm();
        g.h = 2.0;
        meteor_here(&mut g);
        g.step(0.1, Input::default());
        assert_eq!(g.lives, 3);
    }

    #[test]
    fn a_hit_resets_the_combo_and_grants_invulnerability() {
        let mut g = calm();
        crystal_here(&mut g, false);
        g.step(DT, Input::default());
        assert_eq!(g.chain, 1);
        meteor_here(&mut g);
        g.step(0.1, Input::default());
        assert_eq!((g.chain, g.lives, g.flawless), (0, 2, false));
        meteor_here(&mut g);
        g.step(0.1, Input::default());
        assert_eq!(g.lives, 2); // 1,5 s d'invulnérabilité
    }

    #[test]
    fn blast_destroys_crystals_and_drones_in_its_radius() {
        let mut g = calm();
        let dir = Vec3::X; // à 14 unités du joueur : hors de portée du souffle
        g.crystals.push(Crystal { dir, life: 5.0, gold: false });
        g.drones.push(Drone { n: dir, f: Vec3::Y, arm: 0.0 });
        g.meteors.push(Meteor { dir, alt: 0.01, spin: 0.0 });
        g.step(0.1, Input::default());
        assert!(g.crystals.iter().all(|c| arc(c.dir, dir) >= BLAST));
        assert!(g.drones.is_empty());
        assert_eq!((g.kills, g.score), (1, 20));
        assert_eq!(g.lives, 3);
    }

    #[test]
    fn shock_hurts_on_the_ground_but_not_in_the_air() {
        for (h, lives) in [(0.0, 2), (2.0, 3)] {
            let (mut d, mut f) = (Vec3::Y, -Vec3::Z);
            walk(&mut d, &mut f, 5.0); // centre de l'onde à 5 unités du joueur
            let mut g = calm();
            g.h = h;
            g.shocks.push(Shock { dir: d, r: 4.9, harmful: true });
            g.step(0.02, Input::default());
            assert_eq!(g.lives, lives, "hauteur {h}");
        }
    }

    #[test]
    fn dash_is_fast_invulnerable_and_recharges() {
        let mut g = calm();
        let start = g.n;
        g.step(DT, Input { dash: true, ..Default::default() });
        assert!(g.dash_t > 0.0 && g.dash_cd > 1.0);
        meteor_here(&mut g);
        g.step(DT, Input::default());
        assert_eq!(g.lives, 3); // invulnérable pendant le dash
        for _ in 0..15 {
            g.step(DT, Input::default());
        }
        assert!(arc(start, g.n) > 4.0, "distance {}", arc(start, g.n));
        let (cd, t) = (g.dash_cd, g.dash_t);
        g.step(DT, Input { dash: true, ..Default::default() });
        assert!(g.dash_cd < cd && t == 0.0 && g.dash_t == 0.0, "pas de dash pendant la recharge");
    }

    #[test]
    fn landing_on_a_drone_kills_it_and_bounces() {
        let mut g = calm();
        g.drones.push(Drone { n: g.n, f: g.f, arm: 0.0 });
        g.h = 1.0;
        g.vh = -3.0;
        g.step(DT, Input::default());
        assert!(g.drones.is_empty() && g.vh > 0.0);
        assert_eq!((g.kills, g.lives), (1, 3));
        assert!(g.score >= 30);
    }

    #[test]
    fn touching_an_armed_drone_on_the_ground_hurts_but_not_before_it_arms() {
        let mut g = calm();
        g.drones.push(Drone { n: g.n, f: g.f, arm: 1.0 });
        g.step(DT, Input::default());
        assert_eq!(g.lives, 3);
        let mut g = calm();
        g.drones.push(Drone { n: g.n, f: g.f, arm: 0.0 });
        g.step(DT, Input::default());
        assert_eq!(g.lives, 2);
    }

    #[test]
    fn drones_chase_the_player() {
        let mut g = calm();
        let (mut n, mut f) = (Vec3::Y, -Vec3::Z);
        walk(&mut n, &mut f, 12.0);
        g.drones.push(Drone { n, f: -f, arm: 0.0 }); // tourné dans le mauvais sens
        let before = arc(n, g.n);
        for _ in 0..180 {
            g.step(DT, Input::default());
        }
        assert!(arc(g.drones[0].n, g.n) < before - 3.0 || g.lives < 3);
    }

    #[test]
    fn salvo_makes_a_ring_with_a_safe_center() {
        let mut g = calm();
        g.salvo();
        assert_eq!(g.meteors.len(), 6);
        assert!(g.meteors.iter().all(|m| (arc(m.dir, g.n) - 7.0).abs() < 0.2));
    }

    #[test]
    fn waves_add_dangers_and_ask_for_more() {
        let (a, b) = (cfg(1), cfg(5));
        assert!(!a.shocks && !a.salvos && a.drones == 0 && a.lead == 0.0);
        assert!(b.shocks && b.salvos && b.drones >= 2 && b.lead > 0.4);
        assert!(b.quota > a.quota && b.every < a.every);
    }

    // ---------- simulations d'équilibrage (sans fenêtre) ----------

    /// Joue une partie complète : (vague atteinte, score, secondes jouées).
    fn simulate(seed: u64, pilot: fn(&Game) -> Input, max_secs: f32) -> (u32, u32, f32) {
        let mut g = Game::new(seed);
        let mut t = 0.0;
        while g.lives > 0 && t < max_secs {
            let inp = pilot(&g);
            g.step(DT, inp);
            t += DT;
        }
        (g.wave, g.score, t)
    }

    fn idle(_: &Game) -> Input {
        Input::default()
    }

    /// Ramasse le cristal le plus proche sans jamais rien esquiver.
    fn naive(g: &Game) -> Input {
        let near = g.crystals.iter().min_by(|a, b| arc(a.dir, g.n).total_cmp(&arc(b.dir, g.n)));
        let (steer, ahead) = near.map_or((0.0, 0.0), |c| aim(g.n, g.f, c.dir));
        Input { fwd: ahead > 0.3, steer, ..Default::default() }
    }

    #[test]
    fn standing_still_loses_quickly() {
        for seed in 1..=6 {
            let (wave, _, secs) = simulate(seed, idle, 120.0);
            assert!(wave == 1 && secs < 90.0, "graine {seed} : vague {wave}, {secs:.0} s");
        }
    }

    #[test]
    fn dodging_matters_and_the_game_stays_hard() {
        let mean = |pilot: fn(&Game) -> Input| {
            let waves: Vec<u32> = (1..=6).map(|s| simulate(s, pilot, 400.0).0).collect();
            (waves.iter().sum::<u32>() as f32 / waves.len() as f32, *waves.iter().max().unwrap())
        };
        let ((naive_avg, _), (bot_avg, bot_max)) = (mean(naive), mean(Game::bot));
        assert!(bot_avg > naive_avg + 0.8, "esquiveur {bot_avg:.1} contre naïf {naive_avg:.1}");
        assert!(bot_max < 12, "même le bot esquiveur doit finir par perdre (vague {bot_max})");
    }

    /// Rapport de réglage : `cargo test --release balance_report -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn balance_report() {
        for (name, pilot) in [("inactif", idle as fn(&Game) -> Input), ("naïf", naive), ("esquiveur", Game::bot)] {
            let runs: Vec<_> = (1..=20).map(|s| simulate(s, pilot, 900.0)).collect();
            let avg = |f: fn(&(u32, u32, f32)) -> f32| runs.iter().map(f).sum::<f32>() / runs.len() as f32;
            println!(
                "{name:10} vagues {:?}\n{:10} durée moy. {:.0} s, score moy. {:.0}",
                runs.iter().map(|r| r.0).collect::<Vec<_>>(),
                "",
                avg(|r| r.2),
                avg(|r| r.1 as f32)
            );
        }
    }
}
