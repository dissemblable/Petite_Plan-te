//! Interface néon : panneaux translucides à contour lumineux, textes qui rayonnent, barres en cases.
use crate::draw::{mix, shade, theme};
use crate::game::*;
use crate::sphere::hidden_by_planet;
use macroquad::prelude::*;

const PANEL: Color = Color::new(0.02, 0.03, 0.09, 0.62);
const LABEL: Color = Color::new(0.55, 0.62, 0.8, 1.0);

/// Nom de la vague et astuce sur son nouveau danger (boss ou modificateur à partir de la vague 7).
pub fn wave_text(g: &Game) -> (&'static str, &'static str) {
    if cfg(g.wave).boss {
        return ("BOSS", "Drone géant : attire-le 3 fois sous une météorite pour finir la vague !");
    }
    match g.twist {
        Some(Twist::Fragile) => return ("DALLES FRAGILES", "Ta traînée brûle quelques secondes : ne repasse pas dessus !"),
        Some(Twist::LowGravity) => return ("GRAVITÉ FAIBLE", "Sauts plus hauts et plus longs : vise bien pour écraser les drones."),
        Some(Twist::Eclipse) => return ("ÉCLIPSE", "La face cachée s'éteint : seuls les marquages au sol restent visibles."),
        Some(Twist::Fleeing) => return ("CRISTAUX FUYANTS", "Les cristaux s'enfuient quand tu approches : coupe-leur la route !"),
        None => {}
    }
    match g.wave {
        1 => ("PLUIE", "Ramasse les cristaux et évite les zones d'impact."),
        2 => ("ONDES", "Les impacts lancent des ondes : saute par-dessus ! Le cristal doré est risqué."),
        3 => ("CHASSEURS", "Retombe sur un drone pour l'écraser, ou attire-le sous une météorite."),
        4 => ("SALVES", "Les salves t'encerclent : sors vite, ou saute quand les ondes se rejoignent."),
        5 => ("ORDRE", "Cyan, rose, vert dans l'ordre ! Un mauvais coûte une case : saute par-dessus."),
        _ => ("CHAOS", "Plus de drones, plus de météorites, qui visent là où tu vas : change de cap !"),
    }
}

/// Dernier coup reçu : ce qui l'a donné, et comment l'éviter la prochaine fois.
fn cause_text(c: Cause) -> (&'static str, &'static str) {
    match c {
        Cause::Meteor => ("Touché par une météorite", "Sors de la zone rouge avant l'impact, ou saute juste avant."),
        Cause::Shock => ("Touché par une onde", "Saute quand l'anneau arrive sur toi, même en le fuyant."),
        Cause::Drone => ("Touché par un drone", "Saute, puis retombe dessus pour l'écraser."),
        Cause::Trail => ("Brûlé par ta traînée", "Dalles fragiles : ne repasse pas sur ta trace tant qu'elle est rouge."),
    }
}

fn ease(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

fn width(s: &str, size: f32) -> f32 {
    measure_text(s, None, size as u16, 1.0).width
}

/// Texte qui rayonne : halo (copies décalées et translucides) puis texte net, un peu éclairci.
fn glow_text(s: &str, x: f32, y: f32, size: f32, col: Color) {
    for (dx, dy) in [(-2.0, 0.0), (2.0, 0.0), (0.0, -2.0), (0.0, 2.0)] {
        draw_text(s, x + dx, y + dy, size, Color { a: 0.2 * col.a, ..col });
    }
    draw_text(s, x, y, size, Color { a: col.a, ..mix(col, WHITE, 0.3) });
}

fn say(s: &str, y: f32, size: f32, col: Color) {
    draw_text(s, (screen_width() - width(s, size)) / 2.0, y, size, col);
}

fn glow_say(s: &str, y: f32, size: f32, col: Color) {
    glow_text(s, (screen_width() - width(s, size)) / 2.0, y, size, col);
}

/// Panneau translucide sombre, contour lumineux et coins repérés.
fn panel(x: f32, y: f32, w: f32, h: f32, col: Color) {
    draw_rectangle(x, y, w, h, PANEL);
    draw_rectangle_lines(x, y, w, h, 2.0, Color { a: 0.3, ..col });
    for (cx, cy, sx, sy) in [(x, y, 1.0, 1.0), (x + w, y, -1.0, 1.0), (x, y + h, 1.0, -1.0), (x + w, y + h, -1.0, -1.0)] {
        draw_line(cx, cy, cx + 12.0 * sx, cy, 3.0, col);
        draw_line(cx, cy, cx, cy + 12.0 * sy, 3.0, col);
    }
}

/// Petit cube (une vie) : face lumineuse, reflet, contour clair.
fn cube(x: f32, y: f32, s: f32, col: Color) {
    draw_rectangle(x, y, s, s, shade(col, 0.5));
    draw_rectangle(x + s * 0.14, y + s * 0.14, s * 0.72, s * 0.72, col);
    draw_rectangle(x + s * 0.24, y + s * 0.24, s * 0.2, s * 0.2, Color { a: 0.85, ..WHITE });
    draw_rectangle_lines(x, y, s, s, 2.0, mix(col, WHITE, 0.5));
}

/// Icône d'un cristal de couleur `kind`, centrée en (x, y), de la même forme que lui : prisme, cube, losange.
fn gem(kind: usize, x: f32, y: f32, s: f32) {
    let (col, edge) = (KINDS[kind], mix(KINDS[kind], WHITE, 0.5));
    match kind {
        1 => cube(x - s / 2.0, y - s / 2.0, s, col),
        2 => {
            draw_poly(x, y, 4, s * 0.6, 0.0, shade(col, 0.5));
            draw_poly(x, y, 4, s * 0.45, 0.0, col);
            draw_poly_lines(x, y, 4, s * 0.6, 0.0, 2.0, edge);
        }
        _ => {
            let (w, h) = (s * 0.45, s);
            draw_rectangle(x - w / 2.0, y - h / 2.0, w, h, shade(col, 0.5));
            draw_rectangle(x - w * 0.32, y - h * 0.42, w * 0.64, h * 0.84, col);
            draw_rectangle_lines(x - w / 2.0, y - h / 2.0, w, h, 2.0, edge);
        }
    }
}

/// Barre en cases : `filled` cases allumées sur `total` ; la dernière allumée pulse.
fn cells(r: Rect, filled: u32, total: u32, col: Color, t: f32) {
    let gap = 3.0;
    let cw = ((r.w - gap * total.saturating_sub(1) as f32) / total.max(1) as f32).max(2.0);
    for i in 0..total {
        let cx = r.x + i as f32 * (cw + gap);
        if i < filled {
            let pulse = if i + 1 == filled { 0.75 + 0.25 * (t * 8.0).sin() } else { 1.0 };
            draw_rectangle(cx - 1.0, r.y - 1.0, cw + 2.0, r.h + 2.0, Color { a: 0.18, ..col });
            draw_rectangle(cx, r.y, cw, r.h, shade(col, pulse));
        } else {
            draw_rectangle(cx, r.y, cw, r.h, Color::new(1.0, 1.0, 1.0, 0.12));
        }
    }
}

/// Interface de jeu : score, vies, dash, vague et quota, record, combo, bannière de vague.
pub fn hud(g: &Game, best: u32) {
    let (w, t, now) = (screen_width(), theme(g.wave), g.t);
    let c = cfg(g.wave);

    // score, vies, recharge du dash
    panel(16.0, 14.0, 280.0, 126.0, CYAN);
    draw_text("SCORE", 30.0, 40.0, 20.0, LABEL);
    glow_text(&g.score.to_string(), 30.0, 82.0, 44.0, WHITE);
    for i in 0..MAX_LIVES {
        let x = 30.0 + i as f32 * 26.0;
        if i < g.lives {
            cube(x, 94.0, 20.0, HOTPINK);
        } else {
            draw_rectangle_lines(x, 94.0, 20.0, 20.0, 2.0, Color { a: 0.25, ..LABEL });
        }
    }
    let ready = (1.0 - g.dash_cd / DASH_CD).clamp(0.0, 1.0);
    cells(Rect::new(30.0, 124.0, 150.0, 8.0), (ready * 5.0) as u32, 5, if ready >= 1.0 { CYAN } else { GRAY }, now);
    draw_text("MAJ", 192.0, 133.0, 18.0, LABEL);

    // vague et quota : une case par cristal ; dessous, le bonus doublé « sans faute », qui s'éteint au premier coup
    let (name, _) = wave_text(g);
    let pw = 400.0;
    let px = (w - pw) / 2.0;
    panel(px, 14.0, pw, 86.0, t);
    glow_say(&format!("VAGUE {} : {name}", g.wave), 46.0, 28.0, t);
    cells(Rect::new(px + 20.0, 58.0, pw - 40.0, 10.0), g.got.min(c.quota), c.quota, t, now);
    if g.flawless {
        say("SANS FAUTE : BONUS x2", 90.0, 20.0, AMBER);
    } else {
        say("SANS FAUTE : PERDU", 90.0, 20.0, Color { a: 0.35, ..LABEL });
    }

    // ordre des couleurs (vague 5 et plus) : le cristal attendu en grand, puis les suivants
    if c.kinds > 1 {
        let x = w / 2.0 - 100.0;
        panel(x, 108.0, 200.0, 50.0, KINDS[g.need]);
        draw_text("ORDRE", x + 16.0, 140.0, 20.0, LABEL);
        for i in 0..c.kinds {
            let (cx, s) = if i == 0 { (x + 110.0, 30.0) } else { (x + 120.0 + 28.0 * i as f32, 18.0) };
            gem((g.need + i) % c.kinds, cx, 133.0, s);
        }
    }

    // points de vie du boss, sous les panneaux du haut, une fois la bannière passée
    if g.rest == 0.0 && g.drones.iter().any(|d| d.boss) {
        glow_say(&format!("BOSS : encore {} météorite{}", g.boss_hp, if g.boss_hp > 1 { "s" } else { "" }), 186.0, 26.0, AMBER);
    }

    // record et combo (multiplicateur, longueur de la chaîne, temps pour la prolonger)
    panel(w - 296.0, 14.0, 280.0, 54.0, AMBER);
    draw_text("RECORD", w - 282.0, 46.0, 20.0, LABEL);
    let record = best.max(g.score).to_string();
    glow_text(&record, w - 28.0 - width(&record, 34.0), 52.0, 34.0, WHITE);
    if g.chain >= 2 {
        panel(w - 216.0, 78.0, 200.0, 92.0, AMBER);
        glow_text(&format!("x{}", g.mult()), w - 200.0, 146.0, 72.0, AMBER);
        draw_text("CHAÎNE", w - 104.0, 108.0, 20.0, LABEL);
        glow_text(&g.chain.to_string(), w - 104.0, 144.0, 40.0, WHITE);
        cells(Rect::new(w - 200.0, 154.0, 168.0, 6.0), (5.0 * g.chain_t / CHAIN_TIME).ceil() as u32, 5, AMBER, now);
    }

    // bannière de vague, pendant la pause du début : glisse, s'attarde, s'efface ; un trait se déploie dessous
    if g.banner > 0.0 {
        let (name, tip) = wave_text(g);
        let shown = rest_time(g.wave) - g.banner; // temps depuis son apparition
        let into = ease(shown / 0.45);
        let a = into * (g.banner / 0.5).min(1.0);
        let dy = (1.0 - into) * 24.0;
        glow_say(&format!("VAGUE {} : {name}", g.wave), 214.0 + dy, 72.0, Color { a, ..t });
        let half = 260.0 * ease(shown / 0.6);
        draw_line(w / 2.0 - half, 232.0, w / 2.0 + half, 232.0, 2.0, Color { a, ..t });
        let tw = width(tip, 30.0);
        draw_rectangle((w - tw) / 2.0 - 18.0, 248.0 + dy, tw + 36.0, 36.0, Color { a: PANEL.a * a, ..PANEL }); // lisible sur la planète
        say(tip, 272.0 + dy, 30.0, Color { a: a * 0.9, ..WHITE });
    }
}

/// Points flottants : projetés de l'espace 3D vers l'écran avec la matrice de la caméra. On n'affiche pas
/// ceux qui sont derrière la planète vue de `eye` : le texte 2D ne passe pas par le test de profondeur.
pub fn popups(g: &Game, view: Mat4, eye: Vec3) {
    let (w, h) = (screen_width(), screen_height());
    for p in &g.popups {
        let ndc = view.project_point3(p.pos);
        if ndc.z.abs() > 1.0 || hidden_by_planet(eye, p.pos) {
            continue; // derrière la caméra, ou derrière la planète
        }
        let col = Color { a: (p.life * 2.0).min(1.0), ..p.col };
        glow_text(&p.text, (ndc.x + 1.0) / 2.0 * w - width(&p.text, 32.0) / 2.0, (1.0 - ndc.y) / 2.0 * h, 32.0, col);
    }
}

/// Invite qui clignote doucement.
fn prompt(s: &str, y: f32, col: Color) {
    let a = 0.6 + 0.4 * (get_time() as f32 * 4.0).sin();
    glow_say(s, y, 36.0, Color { a, ..col });
}

/// Écran titre. `quick` : le départ rapide est débloqué.
pub fn title(quick: bool) {
    let w = screen_width();
    glow_say("PETITE PLANÈTE", 200.0, 96.0, Color { a: 0.88 + 0.12 * (get_time() as f32 * 2.0).sin(), ..CYAN });
    say("Ramasse les cristaux, évite les météorites, enchaîne les vagues", 262.0, 34.0, WHITE);
    // sous l'astronaute, qui est au centre de l'écran
    panel((w - 860.0) / 2.0, 452.0, 860.0, 108.0, CYAN);
    say("Flèches ou ZQSD/WASD : avancer, tourner, faire demi-tour", 482.0, 28.0, LABEL);
    say("ESPACE : sauter     MAJ : dash     P ou ÉCHAP : pause", 514.0, 28.0, LABEL);
    say("G : lueur     ÉCHAP deux fois : quitter", 546.0, 28.0, LABEL);
    if quick {
        say(&format!("ENTRÉE : départ rapide à la vague {QUICK_START}"), 616.0, 28.0, AMBER);
    }
    prompt("Appuie sur ESPACE", 670.0, HOTPINK);
}

pub fn pause() {
    let w = screen_width();
    panel((w - 520.0) / 2.0, 230.0, 520.0, 170.0, CYAN);
    glow_say("PAUSE", 320.0, 80.0, WHITE);
    say("P : reprendre     ÉCHAP : quitter", 372.0, 30.0, LABEL);
}

/// Écran de fin : ce qui a touché le joueur en dernier et comment l'éviter, puis les chiffres de la partie.
/// L'invite n'apparaît que lorsque ESPACE est accepté (`ready`) ; `quick` : le départ rapide est débloqué.
pub fn over(g: &Game, best: u32, new_record: bool, ready: bool, quick: bool) {
    let w = screen_width();
    panel((w - 720.0) / 2.0, 130.0, 720.0, 390.0, HOTPINK);
    glow_say("PERDU !", 220.0, 96.0, HOTPINK);
    say(&format!("Vague {}     Score {}", g.wave, g.score), 290.0, 44.0, WHITE);
    if let Some(cause) = g.last_hit {
        let (what, tip) = cause_text(cause);
        glow_say(what, 346.0, 34.0, AMBER);
        say(tip, 384.0, 26.0, WHITE);
    }
    say(&format!("Chaîne max {}     Drones détruits {}     Dorés {}", g.best_chain, g.kills, g.golds), 430.0, 26.0, LABEL);
    if new_record {
        glow_say("NOUVEAU RECORD !", 488.0, 36.0, AMBER);
    } else {
        say(&format!("Record : {best}"), 488.0, 36.0, LABEL);
    }
    if ready {
        prompt("ESPACE pour rejouer", 640.0, CYAN);
        if quick {
            say(&format!("ENTRÉE : départ rapide à la vague {QUICK_START}"), 684.0, 26.0, AMBER);
        }
    }
}
