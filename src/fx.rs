//! Effets visuels : halos additifs (sprites), lueur néon et vignettes (post-traitement).
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
use macroquad::prelude::*;

// ---------- réglages de la lueur ----------

const THRESHOLD: f32 = 0.5; // luminosité à partir de laquelle un pixel rayonne
const STRENGTH: f32 = 0.9; // intensité de la lueur ajoutée à l'image
const BLUR_PASSES: usize = 2; // allers-retours de flou : plus il y en a, plus la lueur s'étale

pub const VERTEX: &str = "#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

varying lowp vec2 uv;
varying lowp vec4 color;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
}
";

const SPRITE_FRAGMENT: &str = "#version 100
precision lowp float;

varying vec4 color;
varying vec2 uv;

uniform sampler2D Texture;

void main() {
    gl_FragColor = vec4(color.rgb, color.a * texture2D(Texture, uv).a);
}
";

// Les trois passes de la lueur : extraction des pixels brillants (4 échantillons pour limiter le
// scintillement des lignes fines), flou gaussien séparable, ajout sur l'écran.
const EXTRACT_FRAGMENT: &str = "#version 100
precision mediump float;

varying vec4 color;
varying vec2 uv;

uniform sampler2D Texture;
uniform vec2 texel;
uniform float threshold;

void main() {
    vec3 c = texture2D(Texture, uv + texel * vec2(-1.0, -1.0)).rgb
           + texture2D(Texture, uv + texel * vec2( 1.0, -1.0)).rgb
           + texture2D(Texture, uv + texel * vec2(-1.0,  1.0)).rgb
           + texture2D(Texture, uv + texel * vec2( 1.0,  1.0)).rgb;
    c *= 0.25;
    float l = max(c.r, max(c.g, c.b));
    gl_FragColor = vec4(c * smoothstep(threshold, threshold + 0.3, l), 1.0);
}
";

const BLUR_FRAGMENT: &str = "#version 100
precision mediump float;

varying vec4 color;
varying vec2 uv;

uniform sampler2D Texture;
uniform vec2 dir;

void main() {
    vec3 s = texture2D(Texture, uv).rgb * 0.2270270270;
    s += (texture2D(Texture, uv + dir * 1.3846153846).rgb + texture2D(Texture, uv - dir * 1.3846153846).rgb) * 0.3162162162;
    s += (texture2D(Texture, uv + dir * 3.2307692308).rgb + texture2D(Texture, uv - dir * 3.2307692308).rgb) * 0.0702702703;
    gl_FragColor = vec4(s, 1.0);
}
";

const ADD_FRAGMENT: &str = "#version 100
precision mediump float;

varying vec4 color;
varying vec2 uv;

uniform sampler2D Texture;
uniform float strength;

void main() {
    gl_FragColor = vec4(texture2D(Texture, uv).rgb * strength, 1.0);
}
";

/// Lueur néon : tout ce qui est très lumineux dans l'image rayonne autour de lui.
pub struct Bloom {
    screen: RenderTarget, // copie de l'écran, à sa taille
    a: RenderTarget,      // quart de résolution
    b: RenderTarget,
    extract: Material,
    blur: Material,
    add: Material,
}

impl Bloom {
    /// `None` si les shaders ne compilent pas sur cette machine : le jeu continue sans lueur.
    pub fn new() -> Option<Bloom> {
        let material = |fragment: &'static str, uniforms: Vec<UniformDesc>, blend: Option<BlendState>| {
            load_material(
                ShaderSource::Glsl { vertex: VERTEX, fragment },
                MaterialParams { pipeline_params: PipelineParams { color_blend: blend, ..Default::default() }, uniforms, textures: vec![] },
            )
            .ok()
        };
        let additive = BlendState::new(Equation::Add, BlendFactor::One, BlendFactor::One);
        let (w, h) = (screen_width() as u32, screen_height() as u32);
        Some(Bloom {
            screen: render_target(w, h),
            a: render_target((w / 4).max(1), (h / 4).max(1)),
            b: render_target((w / 4).max(1), (h / 4).max(1)),
            extract: material(
                EXTRACT_FRAGMENT,
                vec![UniformDesc::new("texel", UniformType::Float2), UniformDesc::new("threshold", UniformType::Float1)],
                None,
            )?,
            blur: material(BLUR_FRAGMENT, vec![UniformDesc::new("dir", UniformType::Float2)], None)?,
            add: material(ADD_FRAGMENT, vec![UniformDesc::new("strength", UniformType::Float1)], Some(additive))?,
        })
    }

    /// Dessine `src` sur toute la cible `target` avec le matériau `mat`. `uniforms` règle les paramètres
    /// du matériau : ils doivent l'être après `set_camera`, qui vide les dessins en attente, car
    /// macroquad recopie alors les anciens paramètres de ces dessins dans le matériau.
    fn pass(target: &RenderTarget, mat: &Material, src: &Texture2D, flip_y: bool, uniforms: impl FnOnce(&Material)) {
        let (w, h) = (target.texture.width(), target.texture.height());
        set_camera(&Camera2D { zoom: vec2(2.0 / w, 2.0 / h), target: vec2(w / 2.0, h / 2.0), render_target: Some(target.clone()), ..Default::default() });
        uniforms(mat);
        gl_use_material(mat);
        draw_texture_ex(src, 0.0, 0.0, WHITE, DrawTextureParams { dest_size: Some(vec2(w, h)), flip_y, ..Default::default() });
        gl_use_default_material();
    }

    /// À appeler une fois la scène 3D dessinée, avant l'interface.
    pub fn apply(&mut self) {
        let (w, h) = (screen_width(), screen_height());
        if w < 8.0 || h < 8.0 {
            return; // fenêtre réduite : rien à dessiner
        }
        if self.screen.texture.width() != w || self.screen.texture.height() != h {
            let (w, h) = (w as u32, h as u32);
            self.screen = render_target(w, h); // la fenêtre a été redimensionnée
            self.a = render_target((w / 4).max(1), (h / 4).max(1));
            self.b = render_target((w / 4).max(1), (h / 4).max(1));
        }
        set_default_camera(); // termine le dessin en cours avant de copier l'écran
        self.screen.texture.grab_screen();
        let (qw, qh) = (self.a.texture.width(), self.a.texture.height());
        let extract = |m: &Material| {
            m.set_uniform("texel", vec2(1.0 / w, 1.0 / h));
            m.set_uniform("threshold", THRESHOLD);
        };
        Self::pass(&self.a, &self.extract, &self.screen.texture, true, extract); // la copie de l'écran est à l'envers
        for _ in 0..BLUR_PASSES {
            Self::pass(&self.b, &self.blur, &self.a.texture, false, |m| m.set_uniform("dir", vec2(1.0 / qw, 0.0)));
            Self::pass(&self.a, &self.blur, &self.b.texture, false, |m| m.set_uniform("dir", vec2(0.0, 1.0 / qh)));
        }
        set_default_camera();
        self.add.set_uniform("strength", STRENGTH);
        gl_use_material(&self.add);
        draw_texture_ex(&self.a.texture, 0.0, 0.0, WHITE, DrawTextureParams { dest_size: Some(vec2(w, h)), ..Default::default() });
        gl_use_default_material();
    }
}

/// Coins assombris : maillage de 9 × 9 sommets dont l'opacité croît vers les bords.
pub fn vignette() {
    let (w, h, n) = (screen_width(), screen_height(), 8usize);
    let mut vertices = Vec::with_capacity((n + 1) * (n + 1));
    for j in 0..=n {
        for i in 0..=n {
            let (x, y) = (i as f32 / n as f32, j as f32 / n as f32);
            let d = ((x - 0.5) * 2.0).hypot((y - 0.5) * 2.0); // 0 au centre, 1,41 dans les coins
            let a = 0.55 * ((d - 0.6) / 0.8).clamp(0.0, 1.0).powi(2);
            vertices.push(Vertex::new(x * w, y * h, 0.0, 0.0, 0.0, Color::new(0.0, 0.0, 0.03, a)));
        }
    }
    let mut indices = Vec::with_capacity(6 * n * n);
    for j in 0..n {
        for i in 0..n {
            let a = (j * (n + 1) + i) as u16;
            let b = a + (n + 1) as u16;
            indices.extend([a, a + 1, b, a + 1, b + 1, b]);
        }
    }
    draw_mesh(&Mesh { vertices, indices, texture: None });
}

/// Halo : dégradé radial blanc (opaque au centre, transparent au bord), teinté à l'affichage.
pub fn glow_texture() -> Texture2D {
    let mut img = Image::gen_image_color(64, 64, WHITE);
    for y in 0..64u32 {
        for x in 0..64u32 {
            let (dx, dy) = (x as f32 - 31.5, y as f32 - 31.5);
            let r = ((dx * dx + dy * dy).sqrt() / 32.0).min(1.0);
            img.set_pixel(x, y, Color::new(1.0, 1.0, 1.0, (1.0 - r).powf(2.2)));
        }
    }
    Texture2D::from_image(&img)
}

/// Matériau des halos et des décalques lumineux : mélange additif. Il écrit la profondeur (`depth_write`),
/// car miniquad n'active le test de profondeur que dans ce cas : sans lui, ces effets seraient visibles à
/// travers la planète. Conséquence : à dessiner du plus bas au plus haut (décalques) et du plus loin au
/// plus proche (halos), pour qu'ils ne se masquent pas entre eux.
/// `None` si les shaders ne compilent pas sur cette machine : le jeu dessine alors avec le matériau par défaut.
pub fn sprite_material() -> Option<Material> {
    load_material(
        ShaderSource::Glsl { vertex: VERTEX, fragment: SPRITE_FRAGMENT },
        MaterialParams {
            pipeline_params: PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write: true,
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::One,
                )),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .ok()
}
