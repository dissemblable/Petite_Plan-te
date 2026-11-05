//! Petite Planète : un mini-jeu 3D (macroquad).
//! Cours sur une planète, ramasse les cristaux, esquive les dangers, enchaîne les vagues.
mod draw;
mod game;
mod sphere;

use draw::{planet_grid, theme};
use game::*;
use macroquad::prelude::*;

/// Les touches sont physiques : sur AZERTY, W/A/D sont les touches Z/Q/D.
fn keyboard() -> Input {
    let key = |a, b| is_key_down(a) || is_key_down(b);
    Input {
        fwd: key(KeyCode::W, KeyCode::Up),
        steer: key(KeyCode::A, KeyCode::Left) as i32 as f32 - key(KeyCode::D, KeyCode::Right) as i32 as f32,
        jump: is_key_pressed(KeyCode::Space),
        dash: is_key_pressed(KeyCode::LeftShift) || is_key_pressed(KeyCode::RightShift),
    }
}

// ---------- record ----------

const RECORD_FILE: &str = "record.txt"; // à côté de l'endroit d'où on lance le jeu

fn load_record(path: &str) -> u32 {
    std::fs::read_to_string(path).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0)
}

fn save_record(path: &str, score: u32) {
    let _ = std::fs::write(path, score.to_string()); // pas bloquant : dossier en lecture seule, etc.
}

// ---------- interface ----------

/// Nom de la vague et astuce sur son nouveau danger.
fn wave_text(w: u32) -> (&'static str, &'static str) {
    match w {
        1 => ("PLUIE", "Ramasse les cristaux et évite les zones d'impact."),
        2 => ("ONDES", "Les impacts lancent des ondes : saute par-dessus ! Le cristal doré est risqué."),
        3 => ("CHASSEURS", "Retombe sur un drone pour l'écraser, ou attire-le sous une météorite."),
        4 => ("SALVES", "Les salves t'encerclent : reste au centre ou fuis par une brèche."),
        _ => ("CHAOS", "Les météorites visent devant toi. Change de direction !"),
    }
}

fn say(s: &str, y: f32, size: f32, col: Color) {
    let w = measure_text(s, None, size as u16, 1.0).width;
    draw_text(s, (screen_width() - w) / 2.0, y, size, col);
}

fn bar(x: f32, y: f32, w: f32, fill: f32, col: Color) {
    draw_rectangle(x, y, w, 8.0, Color::new(1.0, 1.0, 1.0, 0.15));
    draw_rectangle(x, y, w * fill.clamp(0.0, 1.0), 8.0, col);
}

fn hud(g: &Game, best: u32) {
    let (w, t) = (screen_width(), theme(g.wave));
    draw_text(format!("SCORE  {}", g.score), 24.0, 44.0, 40.0, WHITE);
    for i in 0..g.lives {
        draw_rectangle(24.0 + i as f32 * 30.0, 58.0, 22.0, 22.0, HOTPINK);
    }
    let ready = 1.0 - g.dash_cd / DASH_CD;
    bar(24.0, 92.0, 110.0, ready, if ready >= 1.0 { CYAN } else { GRAY }); // recharge du dash
    draw_text("MAJ", 142.0, 101.0, 20.0, GRAY);

    say(&format!("VAGUE {}", g.wave), 36.0, 32.0, t);
    bar((w - 320.0) / 2.0, 48.0, 320.0, g.got as f32 / cfg(g.wave).quota as f32, t); // quota de la vague

    draw_text(format!("RECORD  {}", best.max(g.score)), w - 220.0, 44.0, 32.0, GRAY);
    if g.chain >= 2 {
        draw_text(format!("x{}", g.mult()), w - 120.0, 110.0, 64.0, AMBER);
        bar(w - 120.0, 120.0, 90.0, g.chain_t / CHAIN_TIME, AMBER); // temps pour enchaîner
    }

    if g.banner > 0.0 {
        let (name, tip) = wave_text(g.wave);
        let a = g.banner.min(1.0);
        say(&format!("VAGUE {} : {name}", g.wave), 210.0, 72.0, Color { a, ..t });
        say(tip, 262.0, 30.0, Color { a, ..WHITE });
    }
}

/// Points flottants : projetés de l'espace 3D vers l'écran avec la matrice de la caméra.
fn popups(g: &Game, view: Mat4) {
    let (w, h) = (screen_width(), screen_height());
    for p in &g.popups {
        let ndc = view.project_point3(p.pos);
        if ndc.z.abs() > 1.0 {
            continue; // derrière la caméra
        }
        let tw = measure_text(&p.text, None, 32, 1.0).width;
        let col = Color { a: (p.life * 2.0).min(1.0), ..p.col };
        draw_text(&p.text, (ndc.x + 1.0) / 2.0 * w - tw / 2.0, (1.0 - ndc.y) / 2.0 * h, 32.0, col);
    }
}

#[derive(PartialEq)]
enum State {
    Title,
    Play,
    Pause,
    Over,
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
    let seed = || (macroquad::miniquad::date::now() * 1000.0) as u64;
    rand::srand(seed()); // hasard réservé aux effets visuels (tremblement de caméra)

    // Outil de test : `petite-planete <play|idle|title|over> [images] [vague] [graine]`.
    // play = pilote automatique, idle = joueur immobile ; enregistre shot-<mode>-w<vague>-<images>.png
    // puis quitte.
    let mut args = std::env::args().skip(1);
    let shot = args.next();
    let mut num = || args.next().and_then(|s| s.parse::<u64>().ok());
    let (shot_at, start_wave, shot_seed) = (num().unwrap_or(240) as u32, num().unwrap_or(1) as u32, num());

    let mut rng = Rng::new(seed());
    let stars: Vec<Vec3> = (0..300).map(|_| rng.dir() * 300.0).collect();
    let grid = planet_grid();
    let mut g = Game::new(shot_seed.unwrap_or_else(seed));
    let mut state = match shot.as_deref() {
        Some("play" | "idle") => State::Play,
        Some("over") => State::Over,
        _ => State::Title,
    };
    if start_wave > 1 {
        g.start_wave(start_wave);
        g.rest = 0.3;
    }
    match state {
        State::Title => g.crystals.clear(), // écran titre : seulement la planète et l'astronaute
        State::Over => (g.score, g.wave, g.lives) = (1234, 4, 0),
        _ => {}
    }

    let mut best = load_record(RECORD_FILE);
    let (mut new_record, mut frames) = (false, 0);

    loop {
        let dt = get_frame_time().min(0.05);
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        match state {
            State::Play if is_key_pressed(KeyCode::P) => state = State::Pause,
            State::Play => {
                let inp = match shot.as_deref() {
                    Some("play") => g.bot(),
                    Some("idle") => Input::default(),
                    _ => keyboard(),
                };
                g.step(dt, inp);
                if g.lives == 0 {
                    g.clear_hazards();
                    g.crystals.clear(); // écran de fin : seulement la planète et l'astronaute
                    new_record = g.score > best;
                    if new_record {
                        best = g.score;
                        save_record(RECORD_FILE, best);
                    }
                    state = State::Over;
                }
            }
            State::Pause => {
                if is_key_pressed(KeyCode::P) {
                    state = State::Play;
                }
            }
            State::Title | State::Over => {
                g.idle(dt);
                if is_key_pressed(KeyCode::Space) {
                    g = Game::new(seed());
                    state = State::Play;
                }
            }
        }

        clear_background(Color::new(0.04, 0.03, 0.12, 1.0));
        let cam = g.camera(dt);
        set_camera(&cam);
        g.draw(&stars, &grid);

        set_default_camera();
        if g.hurt > 0.0 {
            let (w, h) = (screen_width(), screen_height());
            draw_rectangle(0.0, 0.0, w, h, Color::new(1.0, 0.1, 0.1, g.hurt.min(1.0) * 0.25));
        }
        if matches!(state, State::Play | State::Pause) {
            hud(&g, best);
            popups(&g, cam.matrix());
        }
        match state {
            State::Title => {
                say("PETITE PLANÈTE", 200.0, 96.0, CYAN);
                say("Ramasse les cristaux, évite les météorites, enchaîne les vagues", 260.0, 34.0, WHITE);
                say("Flèches ou ZQSD/WASD : bouger     ESPACE : sauter     MAJ : dash     P : pause", 310.0, 28.0, GRAY);
                say("Appuie sur ESPACE", 670.0, 36.0, HOTPINK);
            }
            State::Pause => {
                say("PAUSE", 300.0, 96.0, WHITE);
                say("P : reprendre     ÉCHAP : quitter", 360.0, 36.0, CYAN);
            }
            State::Over => {
                say("PERDU !", 250.0, 96.0, HOTPINK);
                say(&format!("Vague {}     Score {}", g.wave, g.score), 320.0, 44.0, WHITE);
                if new_record {
                    say("NOUVEAU RECORD !", 372.0, 36.0, AMBER);
                } else {
                    say(&format!("Record : {best}"), 372.0, 36.0, GRAY);
                }
                say("ESPACE pour rejouer", 640.0, 36.0, CYAN);
            }
            State::Play => {}
        }

        if let Some(mode) = &shot {
            frames += 1;
            if frames == shot_at {
                get_screen_data().export_png(&format!("shot-{mode}-w{start_wave}-{shot_at}.png"));
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
    fn record_survives_a_restart_and_ignores_garbage() {
        let path = std::env::temp_dir().join("petite-planete-test-record.txt");
        let path = path.to_str().unwrap();
        let _ = std::fs::remove_file(path);
        assert_eq!(load_record(path), 0); // pas de fichier : record à zéro
        save_record(path, 1234);
        assert_eq!(load_record(path), 1234);
        std::fs::write(path, "n'importe quoi").unwrap();
        assert_eq!(load_record(path), 0);
        let _ = std::fs::remove_file(path);
    }
}
