//! Petite Planète : un mini-jeu 3D (macroquad).
//! Cours sur une planète, ramasse les cristaux, esquive les dangers, enchaîne les vagues.
mod draw;
mod fx;
mod game;
mod hud;
mod models;
mod planet;
mod sound;
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
        back: is_key_pressed(KeyCode::S) || is_key_pressed(KeyCode::Down),
    }
}

// ---------- record ----------

const RECORD_FILE: &str = "record.txt"; // à côté de l'endroit d'où on lance le jeu

/// Record : « score vague », le meilleur score et la plus haute vague atteinte (elle débloque le départ rapide).
/// Un ancien fichier, avec le score seul, reste lisible.
fn load_record(path: &str) -> (u32, u32) {
    let s = std::fs::read_to_string(path).unwrap_or_default();
    let mut num = s.split_whitespace().map(|w| w.parse().unwrap_or(0));
    (num.next().unwrap_or(0), num.next().unwrap_or(0))
}

fn save_record(path: &str, score: u32, wave: u32) {
    let _ = std::fs::write(path, format!("{score} {wave}")); // pas bloquant : dossier en lecture seule, etc.
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
        g.drones.push(Drone { n: d, f: d.any_orthonormal_vector(), arm: 0.0, boss: false });
        g.crystals.push(Crystal { dir: d, life: 100.0, gold: true, kind: 0 });
    }
    match state {
        State::Title => g.crystals.clear(), // écran titre : seulement la planète et l'astronaute
        State::Over => {
            (g.score, g.wave, g.lives, g.last_hit) = (1234, 4, 0, Some(Cause::Shock));
            (g.best_chain, g.kills, g.golds) = (9, 3, 2);
        }
        _ => {}
    }

    let mut sounds = sound::Sounds::load().await; // chargés aussi par l'outil de capture, qui ne les joue pas
    let (mut best, mut best_wave) = load_record(RECORD_FILE);
    let (mut new_record, mut frames, mut over_t) = (false, 0, 0.0f32);
    let (mut frame_sum, mut frame_max) = (0.0f32, 0.0f32); // durée des images, pour l'outil de capture

    loop {
        let raw = get_frame_time();
        let dt = raw.min(0.05);
        if frames > 30 {
            frame_sum += raw; // on saute les premières images (compilation des shaders, etc.)
            frame_max = frame_max.max(raw);
        }
        if is_key_pressed(KeyCode::G) {
            scene.bloom_on = !scene.bloom_on;
        }
        match state {
            // ÉCHAP met en pause : beaucoup de joueurs l'utilisent pour ça, la partie n'est pas perdue
            State::Play if is_key_pressed(KeyCode::P) || is_key_pressed(KeyCode::Escape) => state = State::Pause,
            State::Play => {
                let inp = match shot.as_deref() {
                    Some("play") => g.bot(),
                    Some("idle") => Input::default(),
                    _ => keyboard(),
                };
                g.step(dt, inp);
                if let (None, Some(s)) = (&shot, &mut sounds) {
                    s.update(&g, dt);
                }
                if g.lives == 0 {
                    g.clear_hazards();
                    g.crystals.clear(); // écran de fin : seulement la planète et l'astronaute
                    new_record = g.score > best;
                    if new_record || g.wave > best_wave {
                        (best, best_wave) = (best.max(g.score), best_wave.max(g.wave));
                        save_record(RECORD_FILE, best, best_wave);
                    }
                    over_t = 0.0;
                    state = State::Over;
                }
            }
            State::Pause => {
                if is_key_pressed(KeyCode::Escape) {
                    break;
                }
                if is_key_pressed(KeyCode::P) {
                    state = State::Play;
                }
            }
            State::Title | State::Over => {
                if is_key_pressed(KeyCode::Escape) {
                    break;
                }
                g.idle(dt);
                over_t += dt;
                // à l'écran de fin, ESPACE n'est accepté qu'après 1 s : un saut au moment de mourir ne relance pas.
                // ENTRÉE : départ rapide, une fois la vague QUICK_UNLOCK atteinte.
                let quick = best_wave >= QUICK_UNLOCK && is_key_pressed(KeyCode::Enter);
                if (is_key_pressed(KeyCode::Space) || quick) && (state == State::Title || over_t > 1.0) {
                    g = Game::new(seed());
                    if quick {
                        g.start_wave(QUICK_START);
                    }
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
            State::Title => hud::title(best_wave >= QUICK_UNLOCK),
            State::Pause => hud::pause(),
            State::Over => hud::over(&g, best, new_record, over_t > 1.0, best_wave >= QUICK_UNLOCK),
            State::Play => {}
        }

        if let Some(mode) = &shot {
            frames += 1;
            if frames == shot_at {
                let n = (frames - 30).max(1) as f32;
                let (avg, worst) = (1000.0 * frame_sum / n, 1000.0 * frame_max);
                let sons = if sounds.is_some() { "chargés" } else { "indisponibles" };
                println!("{frames} images : {avg:.2} ms en moyenne, {worst:.1} ms au pire ; sons {sons}");
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
        assert_eq!(load_record(path), (0, 0)); // pas de fichier : record à zéro
        save_record(path, 1234, 6);
        assert_eq!(load_record(path), (1234, 6));
        std::fs::write(path, "2530").unwrap(); // ancien format : le score seul
        assert_eq!(load_record(path), (2530, 0));
        std::fs::write(path, "n'importe quoi").unwrap();
        assert_eq!(load_record(path), (0, 0));
        let _ = std::fs::remove_file(path);
    }
}
