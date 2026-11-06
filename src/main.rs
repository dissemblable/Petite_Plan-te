//! Petite Planète : un mini-jeu 3D (macroquad).
//! Cours sur une planète, ramasse les cristaux, esquive les dangers, enchaîne les vagues.
mod draw;
mod fx;
mod game;
mod hud;
mod models;
mod planet;
mod sphere;

use draw::Scene;
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

    // Outil de test : `petite-planete <play|idle|pause|title|over> [images] [vague] [graine] [nobloom] [got=0.8]`.
    // play = pilote automatique, idle = joueur immobile ; enregistre shot-<mode>-w<vague>-<images>.png
    // puis quitte et affiche la durée des images.
    let mut args = std::env::args().skip(1);
    let shot = args.next();
    let mut num = || args.next().and_then(|s| s.parse::<u64>().ok());
    let (shot_at, start_wave, shot_seed) = (num().unwrap_or(240) as u32, num().unwrap_or(1) as u32, num());
    // arguments facultatifs : `nobloom` (sans lueur), `got=0.8` (80 % du quota déjà ramassé), `far` (dangers à
    // l'opposé du joueur)
    let flags: Vec<String> = args.collect();
    let no_bloom = flags.iter().any(|f| f == "nobloom");
    let got: f32 = flags.iter().find_map(|f| f.strip_prefix("got=")?.parse().ok()).unwrap_or(0.0);

    let mut scene = Scene::new(seed());
    scene.bloom_on = !no_bloom;
    let mut g = Game::new(shot_seed.unwrap_or_else(seed));
    let mut state = match shot.as_deref() {
        Some("play" | "idle") => State::Play,
        Some("pause") => State::Pause,
        Some("over") => State::Over,
        _ => State::Title,
    };
    if start_wave > 1 {
        g.start_wave(start_wave);
        g.rest = 0.3;
    }
    g.got = (cfg(g.wave).quota as f32 * got) as u32;
    if flags.iter().any(|f| f == "far") {
        // des dangers à l'opposé du joueur, pour vérifier qu'on ne les voit pas à travers la planète
        let d = -g.n;
        g.rest = 1e6;
        g.shocks.push(Shock { dir: d, r: 3.0, harmful: true });
        g.meteors.push(Meteor { dir: d, alt: 12.0, spin: 0.0 });
        g.drones.push(Drone { n: d, f: d.any_orthonormal_vector(), arm: 0.0 });
        g.crystals.push(Crystal { dir: d, life: 100.0, gold: true, kind: 0 });
    }
    match state {
        State::Title => g.crystals.clear(), // écran titre : seulement la planète et l'astronaute
        State::Over => (g.score, g.wave, g.lives) = (1234, 4, 0),
        _ => {}
    }

    let mut best = load_record(RECORD_FILE);
    let (mut new_record, mut frames) = (false, 0);
    let (mut frame_sum, mut frame_max) = (0.0f32, 0.0f32); // durée des images, pour l'outil de capture

    loop {
        let raw = get_frame_time();
        let dt = raw.min(0.05);
        if frames > 30 {
            frame_sum += raw; // on saute les premières images (compilation des shaders, etc.)
            frame_max = frame_max.max(raw);
        }
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        if is_key_pressed(KeyCode::G) {
            scene.bloom_on = !scene.bloom_on;
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

        scene.update(&g, dt);
        clear_background(Color::new(0.04, 0.03, 0.12, 1.0));
        let cam = g.camera(dt, scene.roll);
        set_camera(&cam);
        scene.draw(&g, &cam);
        scene.post();

        if g.hurt > 0.0 {
            let (w, h) = (screen_width(), screen_height());
            draw_rectangle(0.0, 0.0, w, h, Color::new(1.0, 0.1, 0.1, g.hurt.min(1.0) * 0.25));
        }
        if matches!(state, State::Play | State::Pause) {
            hud::hud(&g, best);
            hud::popups(&g, cam.matrix(), cam.position);
        }
        match state {
            State::Title => hud::title(),
            State::Pause => hud::pause(),
            State::Over => hud::over(&g, best, new_record),
            State::Play => {}
        }

        if let Some(mode) = &shot {
            frames += 1;
            if frames == shot_at {
                let n = (frames - 30).max(1) as f32;
                println!("{frames} images : {:.2} ms en moyenne, {:.1} ms au pire", 1000.0 * frame_sum / n, 1000.0 * frame_max);
                let suffix = if no_bloom { "-nobloom" } else { "" };
                let mut img = get_screen_data();
                for px in img.get_image_data_mut() {
                    px[3] = 255; // le mélange des vignettes baisse l'alpha du tampon : on l'ignore dans la capture
                }
                img.export_png(&format!("shot-{mode}-w{start_wave}-{shot_at}{suffix}.png"));
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
