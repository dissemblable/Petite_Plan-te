//! Modèles en cubes : astronaute, cristaux, météorites et drones. Les halos sont ajoutés à une liste
//! de sprites que la scène dessine en dernier.
use crate::draw::*;
use crate::game::*;
use crate::sphere::*;
use macroquad::prelude::*;
use std::f32::consts::TAU;

const SUIT: Color = Color::new(0.62, 0.70, 0.85, 1.0); // assez sombre pour ne pas éblouir avec la lueur
const GEAR: Color = Color::new(0.14, 0.16, 0.24, 1.0);

pub fn astronaut(g: &Game, sprites: &mut Vec<Sprite>) {
    if g.hurt > 0.0 && (g.t * 14.0) as i32 % 2 != 0 {
        return; // clignote quand il est invulnérable
    }
    let (n, f) = (g.n, g.f);
    let r = f.cross(n);
    let s = 1.3; // échelle de l'astronaute
    let sy = (1.0 + 0.025 * g.vh.clamp(-8.0, 8.0)).clamp(0.8, 1.25); // étirement vertical au saut
    let lean = Quat::from_axis_angle(r, -0.45 * g.dash_t / DASH_TIME); // penché en avant pendant le dash
    let p = g.pos();
    let air = g.h > 0.05;
    let stride = if g.moving && !air { (g.stride * 1.9).sin() * 0.75 } else { 0.0 };
    let breath = if g.moving || air { 0.0 } else { 0.012 * (g.t * 2.0).sin() };

    // coordonnées locales (x droite, y haut, z avant) vers le monde
    let w = |v: Vec3| p + lean * (r * v.x + n * v.y * sy + f * v.z) * s;
    let ax = [lean * r, lean * n, lean * f];
    let size = |h: Vec3| h * s * vec3(1.0, sy, 1.0);
    let part = |c: Vec3, h: Vec3, fill: Color, edge: Color| draw_box(w(c), ax, size(h), fill, edge);
    let lit = |c: Vec3, h: Vec3, col: Color| glow_box(w(c), ax, size(h), col);
    // membre qui pend depuis `joint` et pivote de `swing` autour de l'axe droite (> 0 : vers l'avant)
    let limb = |joint: Vec3, len: f32, h: Vec3, swing: f32, foot: bool| {
        let q = lean * Quat::from_axis_angle(r, swing);
        let down = q * -n * s * sy;
        let c = w(joint) + down * len;
        let a = [q * r, q * n, q * f];
        draw_box(c, a, size(h), SUIT, CYAN);
        if foot {
            draw_box(c + down * (len + 0.03) + q * f * 0.05 * s, a, size(vec3(0.095, 0.04, 0.12)), GEAR, CYAN);
        }
    };

    // torse, ceinture, voyant de poitrine
    part(vec3(0.0, 0.62, 0.0), vec3(0.26, 0.28, 0.17), SUIT, CYAN);
    part(vec3(0.0, 0.38, 0.0), vec3(0.275, 0.04, 0.185), GEAR, CYAN);
    lit(vec3(0.0, 0.68, 0.18), vec3(0.07, 0.04, 0.02), CYAN);
    // casque, visière, antenne dont le bout clignote
    part(vec3(0.0, 1.12 + breath, 0.02), vec3(0.21, 0.19, 0.2), SUIT, CYAN);
    lit(vec3(0.0, 1.13 + breath, 0.21), vec3(0.15, 0.09, 0.03), CYAN);
    part(vec3(0.12, 1.42 + breath, -0.05), vec3(0.012, 0.12, 0.012), GEAR, CYAN);
    let blink = (g.t * 4.0).sin() > 0.3;
    lit(vec3(0.12, 1.56 + breath, -0.05), Vec3::splat(0.03), if blink { HOTPINK } else { shade(HOTPINK, 0.25) });
    // sac à dos et réacteurs, allumés quand le dash est prêt : pas besoin de quitter l'astronaute des yeux
    part(vec3(0.0, 0.66, -0.26), vec3(0.17, 0.25, 0.1), shade(HOTPINK, 0.55), HOTPINK);
    for x in [-0.09, 0.09] {
        if g.dash_cd == 0.0 {
            lit(vec3(x, 0.36, -0.33), vec3(0.04, 0.05, 0.04), CYAN);
        } else {
            part(vec3(x, 0.36, -0.33), vec3(0.04, 0.05, 0.04), GEAR, HOTPINK);
        }
    }
    // jambes et bras : ils se balancent avec la distance parcourue ; repliés et levés en l'air
    let (leg_l, leg_r) = if air { (0.55, -0.45) } else { (stride, -stride) };
    let (arm_l, arm_r) = if air { (2.3, 2.3) } else { (-stride * 0.8, stride * 0.8) };
    limb(vec3(-0.12, 0.40, 0.0), 0.2, vec3(0.085, 0.2, 0.09), leg_l, true);
    limb(vec3(0.12, 0.40, 0.0), 0.2, vec3(0.085, 0.2, 0.09), leg_r, true);
    limb(vec3(-0.34, 0.84, 0.0), 0.17, vec3(0.07, 0.17, 0.07), arm_l, false);
    limb(vec3(0.34, 0.84, 0.0), 0.17, vec3(0.07, 0.17, 0.07), arm_r, false);

    // flamme de jetpack : pendant le dash, plus faible quand on monte en sautant
    let flame = (g.dash_t / DASH_TIME).max(if g.h > 0.0 && g.vh > 1.0 { 0.5 } else { 0.0 });
    if flame > 0.0 {
        for x in [-0.09, 0.09] {
            for j in 0..3 {
                let wob = 0.8 + 0.4 * (g.t * 60.0 + j as f32 * 2.0 + x * 40.0).sin();
                let col = [mix(CYAN, WHITE, 0.6), CYAN, FIRE][j];
                let size = (0.05 - 0.012 * j as f32) * wob * (0.6 + 0.6 * flame);
                lit(vec3(x, 0.36, -0.38 - 0.14 * j as f32 * (0.5 + flame)), Vec3::splat(size), col);
            }
        }
        sprites.push(Sprite { pos: w(vec3(0.0, 0.36, -0.55)), size: 1.1 * flame + 0.3, col: Color { a: 0.6 * flame, ..CYAN } });
    }
    sprites.push(Sprite { pos: w(vec3(0.0, 1.13 + breath, 0.3)), size: 0.55, col: Color { a: 0.45, ..CYAN } }); // reflet de la visière
}

pub fn crystal(g: &Game, i: usize, k: &Crystal, sprites: &mut Vec<Sprite>) {
    if k.life < 3.0 && (g.t * 8.0) as i32 % 2 == 0 {
        return; // clignote avant de disparaître
    }
    let (col, size, spin, orbs) = if k.gold { (AMBER, 1.25, 4.0, 4) } else { (KINDS[k.kind], 1.0, 2.0, 2) };
    let c = k.dir;
    let q = Quat::from_axis_angle(c, g.t * spin + i as f32);
    let (u, w) = (q * c.any_orthonormal_vector(), q * c.cross(c.any_orthonormal_vector()));
    let base = c * (R + 1.7 + (g.t * 3.0 + i as f32 * 2.0).sin() * 0.15);
    let edge = mix(col, WHITE, 0.75);
    // faisceau visible depuis l'autre côté de l'horizon : vif pour un cristal à prendre, pâle pour les autres
    let wanted = k.gold || k.kind == g.need;
    draw_line_3d(c * R, c * (R + 12.0), Color { a: if wanted { 0.5 } else { 0.1 }, ..col });
    // une forme par couleur, pour ne pas dépendre de la seule couleur : prisme, cube, cube posé sur la pointe
    let tip = Quat::from_rotation_arc((u + c + w).normalize(), c); // la grande diagonale du cube à la verticale
    let (half, ax) = match k.kind {
        1 => (vec3(0.42, 0.42, 0.42), [u, c, w]),
        2 => (vec3(0.4, 0.4, 0.4), [tip * u, tip * c, tip * w]),
        _ => (vec3(0.26, 0.8, 0.26), [u, c, w]),
    };
    draw_box(base, ax, half * size, col, edge);
    // deux éclats plus petits, penchés vers l'extérieur
    let (q1, q2) = (Quat::from_axis_angle(w, 0.4), Quat::from_axis_angle(u, 0.45));
    draw_box(base + (u * 0.36 - c * 0.38) * size, [q1 * u, q1 * c, q1 * w], vec3(0.15, 0.45, 0.15) * size, shade(col, 0.9), edge);
    draw_box(base + (-w * 0.32 - c * 0.45) * size, [q2 * u, q2 * c, q2 * w], vec3(0.12, 0.36, 0.12) * size, shade(col, 0.9), edge);
    // petits cubes en orbite
    for j in 0..orbs {
        let a = g.t * 2.6 + j as f32 * TAU / orbs as f32;
        let pos = base + (u * a.cos() + w * a.sin()) * 0.8 * size + c * 0.35 * (g.t * 3.0 + j as f32).sin();
        glow_box(pos, [u, c, w], Vec3::splat(0.07 * size), mix(col, WHITE, 0.5));
    }
    sprites.push(Sprite { pos: base, size: 2.0 * size, col: Color { a: 0.5 + 0.15 * (g.t * 4.0 + i as f32).sin(), ..col } });
}

pub fn meteor(g: &Game, m: &Meteor, sprites: &mut Vec<Sprite>) {
    let p = m.dir * (R + m.alt);
    let q = Quat::from_euler(EulerRot::XYZ, g.t * 2.1 + m.spin, g.t * 1.3 + m.spin * 2.0, m.spin);
    let ax = [q * Vec3::X, q * Vec3::Y, q * Vec3::Z];
    let rock = Color::new(0.26, 0.14, 0.11, 1.0);
    // rocher : un gros bloc et trois plus petits soudés dessus, arêtes de lave
    draw_box(p, ax, Vec3::splat(1.0), rock, FIRE);
    for (off, h) in [(vec3(0.95, 0.3, 0.2), 0.55), (vec3(-0.75, -0.6, 0.35), 0.5), (vec3(0.1, 0.8, -0.85), 0.45)] {
        draw_box(p + q * off, ax, Vec3::splat(h), rock, FIRE);
    }
    // traînée de braises (vers le haut), de plus en plus petite et sombre
    for k in 1..=8 {
        let t = k as f32 / 9.0;
        let wobble = vec3(
            (g.t * 17.0 + k as f32).sin(),
            (g.t * 13.0 + k as f32 * 2.0).cos(),
            (g.t * 11.0 + k as f32 * 3.0).sin(),
        ) * 0.15
            * k as f32;
        let col = if k <= 2 {
            Color::new(1.0, 0.9, 0.45, 1.0)
        } else if k <= 5 {
            FIRE
        } else {
            Color::new(0.85, 0.15, 0.05, 1.0)
        };
        draw_cube(p + m.dir * (0.9 * k as f32) + wobble, Vec3::splat(1.0 - t), None, Color { a: 1.0 - 0.8 * t, ..col });
    }
    sprites.push(Sprite { pos: p, size: 3.2, col: Color { a: 0.5, ..FIRE } });
}

pub fn drone(g: &Game, d: &Drone, sprites: &mut Vec<Sprite>) {
    let ready = d.arm <= 0.0;
    let eye = if ready || (g.t * 12.0) as i32 % 2 == 0 { DRONE } else { WHITE }; // blanc qui clignote pendant l'armement
    let (n, f) = (d.n, d.f);
    let p = n * (R + 0.8 + 0.1 * (g.t * 5.0).sin());
    let r = f.cross(n);
    let ax = [r, n, f];
    // anneau pointillé au sol : la zone de contact ; faisceau rouge pour le repérer au-delà de l'horizon
    dashed_ring(n, 0.9, 0.09, Color { a: if ready { 0.7 } else { 0.25 }, ..DRONE }, g.t * 5.0);
    draw_line_3d(n * R, n * (R + 12.0), Color { a: if ready { 0.5 } else { 0.2 }, ..DRONE });
    draw_box(p, ax, vec3(0.42, 0.14, 0.42), GEAR, DRONE);
    draw_box(p + n * 0.2, ax, vec3(0.25, 0.1, 0.25), GEAR, DRONE);
    glow_box(p + f * 0.43 + n * 0.02, ax, vec3(0.15, 0.07, 0.05), eye);
    // quatre bras et leurs hélices
    for (k, dir) in [r + f, r - f, -r + f, -r - f].iter().map(|v| v.normalize()).enumerate() {
        let side = n.cross(dir);
        draw_box(p + dir * 0.5, [dir, n, side], vec3(0.3, 0.03, 0.04), GEAR, DRONE);
        let spin = Quat::from_axis_angle(n, g.t * 14.0 + k as f32);
        draw_box(p + dir * 0.85 + n * 0.07, [spin * dir, n, spin * side], vec3(0.22, 0.012, 0.035), WHITE, DRONE);
    }
    // antenne dont le bout clignote
    draw_box(p + n * 0.42 - f * 0.1, ax, vec3(0.012, 0.14, 0.012), GEAR, DRONE);
    let tip = if (g.t * 3.0 + p.x).sin() > 0.0 { DRONE } else { shade(DRONE, 0.3) };
    glow_box(p + n * 0.58 - f * 0.1, ax, Vec3::splat(0.035), tip);
    sprites.push(Sprite { pos: p + f * 0.45, size: 1.1, col: Color { a: 0.6, ..eye } });
}
