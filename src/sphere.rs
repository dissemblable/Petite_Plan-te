//! Géométrie sur la sphère : distances et déplacements à la surface de la planète.
use macroquad::prelude::*;
use std::f32::consts::TAU;

pub const R: f32 = 9.0; // rayon de la planète

/// Distance à la surface entre deux directions unitaires.
pub fn arc(a: Vec3, b: Vec3) -> f32 {
    a.dot(b).clamp(-1.0, 1.0).acos() * R
}

/// Avance de `d` unités le long du grand cercle qui part vers `f`.
pub fn walk(n: &mut Vec3, f: &mut Vec3, d: f32) {
    let q = Quat::from_axis_angle(n.cross(*f).normalize(), d / R);
    *n = (q * *n).normalize();
    *f = q * *f;
    *f = (*f - *n * f.dot(*n)).normalize(); // évite la dérive numérique
}

/// Tourne le regard de `a` radians autour de la verticale (négatif = droite).
pub fn turn(n: Vec3, f: &mut Vec3, a: f32) {
    *f = Quat::from_axis_angle(n, a) * *f;
}

/// Cercle de rayon `rad` (sur la surface) autour de `dir`, en directions unitaires.
pub fn circle(dir: Vec3, rad: f32) -> Vec<Vec3> {
    let (u, t) = (dir.any_orthonormal_vector(), rad / R);
    let v = dir.cross(u);
    (0..=32)
        .map(|i| {
            let a = i as f32 / 32.0 * TAU;
            dir * t.cos() + (u * a.cos() + v * a.sin()) * t.sin()
        })
        .collect()
}

/// Vrai si le point `p` est caché derrière la planète vu de `eye` : le segment de l'œil au point traverse la
/// sphère. Sert aux éléments 2D (points flottants) qui ne passent pas par le test de profondeur.
pub fn hidden_by_planet(eye: Vec3, p: Vec3) -> bool {
    let d = p - eye;
    let (a, b, c) = (d.length_squared(), 2.0 * eye.dot(d), eye.length_squared() - R * R);
    let disc = b * b - 4.0 * a * c;
    if a == 0.0 || disc <= 0.0 {
        return false;
    }
    let t = (-b - disc.sqrt()) / (2.0 * a); // première rencontre avec la sphère, de 0 (l'œil) à 1 (le point)
    t > 0.0 && t < 1.0 - 1e-3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_behind_the_planet_are_hidden() {
        let eye = Vec3::Y * 19.0;
        assert!(hidden_by_planet(eye, -Vec3::Y * (R + 2.0))); // à l'opposé
        assert!(hidden_by_planet(eye, Vec3::X * R)); // au-delà de l'horizon
        assert!(!hidden_by_planet(eye, Vec3::Y * (R + 2.0))); // sous l'œil
        assert!(!hidden_by_planet(eye, vec3(7.2, 7.2, 0.0))); // en vue
        assert!(!hidden_by_planet(eye, vec3(12.0, 0.0, 0.0))); // dépasse au-dessus de l'horizon
        assert!(!hidden_by_planet(eye, Vec3::new(6.4, 6.4, 0.0))); // à la surface, côté visible
    }

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
}
