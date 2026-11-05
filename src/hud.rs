//! Interface néon : panneaux translucides à contour lumineux, textes qui rayonnent, barres en cases.
use crate::draw::{mix, shade, theme};
use crate::game::*;
use crate::sphere::hidden_by_planet;
use macroquad::prelude::*;

const PANEL: Color = Color::new(0.02, 0.03, 0.09, 0.62);
const LABEL: Color = Color::new(0.55, 0.62, 0.8, 1.0);

/// Nom de la vague et astuce sur son nouveau danger.
pub fn wave_text(w: u32) -> (&'static str, &'static str) {
    match w {
        1 => ("PLUIE", "Ramasse les cristaux et évite les zones d'impact."),
        2 => ("ONDES", "Les impacts lancent des ondes : saute par-dessus ! Le cristal doré est risqué."),
        3 => ("CHASSEURS", "Retombe sur un drone pour l'écraser, ou attire-le sous une météorite."),
        4 => ("SALVES", "Les salves t'encerclent : reste au centre ou fuis par une brèche."),
        _ => ("CHAOS", "Les météorites visent devant toi. Change de direction !"),
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
    let quota = cfg(g.wave).quota;

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

    // vague et quota : une case par cristal
    let (name, _) = wave_text(g.wave);
    let pw = 400.0;
    let px = (w - pw) / 2.0;
    panel(px, 14.0, pw, 66.0, t);
    glow_say(&format!("VAGUE {} : {name}", g.wave), 46.0, 28.0, t);
    cells(Rect::new(px + 20.0, 58.0, pw - 40.0, 10.0), g.got.min(quota), quota, t, now);

    // record et combo
    panel(w - 296.0, 14.0, 280.0, 54.0, AMBER);
    draw_text("RECORD", w - 282.0, 46.0, 20.0, LABEL);
    let record = best.max(g.score).to_string();
    glow_text(&record, w - 28.0 - width(&record, 34.0), 52.0, 34.0, WHITE);
    if g.chain >= 2 {
        panel(w - 216.0, 78.0, 200.0, 92.0, AMBER);
        glow_text(&format!("x{}", g.mult()), w - 200.0, 146.0, 72.0, AMBER);
        cells(Rect::new(w - 200.0, 154.0, 168.0, 6.0), (5.0 * g.chain_t / CHAIN_TIME).ceil() as u32, 5, AMBER, now);
    }

    // bannière de vague : glisse, s'attarde, s'efface ; un trait se déploie dessous
    if g.banner > 0.0 {
        let (name, tip) = wave_text(g.wave);
        let into = ease((4.0 - g.banner) / 0.45);
        let a = into * g.banner.min(1.0);
        let dy = (1.0 - into) * 24.0;
        glow_say(&format!("VAGUE {} : {name}", g.wave), 214.0 + dy, 72.0, Color { a, ..t });
        let half = 260.0 * ease((4.0 - g.banner) / 0.6);
        draw_line(w / 2.0 - half, 232.0, w / 2.0 + half, 232.0, 2.0, Color { a, ..t });
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

pub fn title() {
    let w = screen_width();
    glow_say("PETITE PLANÈTE", 200.0, 96.0, Color { a: 0.88 + 0.12 * (get_time() as f32 * 2.0).sin(), ..CYAN });
    say("Ramasse les cristaux, évite les météorites, enchaîne les vagues", 262.0, 34.0, WHITE);
    panel((w - 860.0) / 2.0, 292.0, 860.0, 76.0, CYAN);
    say("Flèches ou ZQSD/WASD : bouger     ESPACE : sauter     MAJ : dash", 322.0, 28.0, LABEL);
    say("P : pause     G : lueur     ÉCHAP : quitter", 354.0, 28.0, LABEL);
    prompt("Appuie sur ESPACE", 670.0, HOTPINK);
}

pub fn pause() {
    let w = screen_width();
    panel((w - 520.0) / 2.0, 230.0, 520.0, 170.0, CYAN);
    glow_say("PAUSE", 320.0, 80.0, WHITE);
    say("P : reprendre     ÉCHAP : quitter", 372.0, 30.0, LABEL);
}

pub fn over(g: &Game, best: u32, new_record: bool) {
    let w = screen_width();
    panel((w - 600.0) / 2.0, 150.0, 600.0, 290.0, HOTPINK);
    glow_say("PERDU !", 240.0, 96.0, HOTPINK);
    say(&format!("Vague {}     Score {}", g.wave, g.score), 312.0, 44.0, WHITE);
    if new_record {
        glow_say("NOUVEAU RECORD !", 376.0, 36.0, AMBER);
    } else {
        say(&format!("Record : {best}"), 376.0, 36.0, LABEL);
    }
    prompt("ESPACE pour rejouer", 640.0, CYAN);
}
