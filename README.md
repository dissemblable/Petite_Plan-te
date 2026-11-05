# Petite Planète

Jeu d'arcade en 3D écrit en Rust avec [macroquad](https://macroquad.rs). Un astronaute court sur une mini-planète faite de dalles cubiques, ramasse des cristaux d'énergie et survit à des vagues de dangers de plus en plus variés : météorites, ondes de choc, drones chasseurs, salves en anneau.

Le style est néo-futuriste et cubique : planète dont les dalles s'allument au fil de la vague, lueur néon, interface à panneaux lumineux. Tout est généré par le code : aucun fichier image, son ou police à installer, et une seule dépendance (`macroquad`).

## Lancer le jeu

Il faut Rust, édition 2024 (version 1.85 ou plus récente) : <https://rustup.rs>. Sous Windows, il faut aussi les outils de compilation C++ de Visual Studio. Le jeu a été développé et testé sous Windows 11.

```
cargo run --release
```

Le mode `--release` est recommandé : en mode debug, le jeu est nettement plus lent.

## Commandes

| Action | Touches |
| --- | --- |
| Avancer | `↑` ou `W` (`Z` sur un clavier AZERTY) |
| Tourner | `←` `→` ou `A` `D` (`Q` `D` sur un clavier AZERTY) |
| Sauter | `Espace` |
| Dash | `Maj` |
| Pause | `P` |
| Couper ou remettre la lueur | `G` |
| Quitter | `Échap` |

`Espace` lance aussi une partie depuis l'écran titre et la relance depuis l'écran de fin. Les touches sont lues par leur position physique : sur AZERTY, les touches `W` `A` `D` d'un clavier QWERTY sont `Z` `Q` `D`.

## Règles

**Les vagues.** Chaque vague demande de ramasser un quota de cristaux : `6 + 2 × numéro de vague`. La planète s'allume dalle par dalle à mesure que le quota se remplit. À la fin d'une vague : 3,5 s de calme, +1 vie (5 au maximum) et un bonus de `100 × numéro de vague`, doublé si aucun coup n'a été reçu pendant la vague. Chaque vague ajoute un danger aux précédentes :

| Vague | Nom | Nouveauté |
| --- | --- | --- |
| 1 | Pluie | Des météorites tombent. Leur zone d'impact apparaît au sol environ 2 s avant la chute. Être dans la zone, au sol, à l'impact coûte une vie. Sauter au bon moment esquive. |
| 2 | Ondes | Chaque impact lance une onde circulaire qu'il faut franchir en sautant. Des cristaux dorés apparaissent au centre des zones d'impact : appât risqué. |
| 3 | Chasseurs | Des drones poursuivent le joueur (moins vite que lui). Retomber dessus après un saut les écrase. Une météorite les tue aussi. |
| 4 | Salves | Toutes les 10 à 16 s, six impacts en anneau autour du joueur. Le centre est sûr, il reste une brèche pour sortir. |
| 5 et plus | Chaos | Tout à la fois, et une part croissante des météorites vise l'endroit où le joueur sera dans 2 s. |

Au fil des vagues, les météorites tombent plus souvent (intervalle `max(0,7 ; 2,2 − 0,2 × vague)` secondes) et il y a plus de drones (4 au maximum).

**Score et combo.** Un cristal vaut 10 points, un cristal doré 50 (et compte pour 3 dans le quota), un drone écrasé 30, un drone tué par une météorite 20. Ramasser ou écraser dans les 3 s qui suivent le précédent fait monter le multiplicateur de ×1 à ×5 ; un coup le remet à zéro. Les cristaux disparaissent au bout de 11 s (7 s pour les dorés).

**Vies et gestes.** On commence avec 3 vies. Un coup donne 1,5 s d'invulnérabilité. Le dash (0,2 s, rechargé en 1,4 s) rend invulnérable pendant qu'il dure. Un saut demandé juste avant l'atterrissage est mémorisé.

**Record.** Le meilleur score est enregistré dans `record.txt`, dans le dossier d'où on lance le jeu.

## Développement

### Organisation du code

| Fichier | Rôle |
| --- | --- |
| `src/main.rs` | Fenêtre, états (titre, jeu, pause, fin), clavier, record, outil de capture. |
| `src/game.rs` | Toute la simulation, sans dessin : règles, vagues, dangers, pilote automatique. Les réglages de difficulté sont en tête de fichier. |
| `src/sphere.rs` | Géométrie sur la sphère : distances, déplacements, cercles, masquage par la planète. |
| `src/planet.rs` | Planète en 384 dalles : éclairage, allumage, assombrissement près des dangers. |
| `src/models.rs` | Astronaute, cristaux, météorites et drones, assemblés à partir de cubes. |
| `src/draw.rs` | Primitives de dessin, scène, caméra. |
| `src/fx.rs` | Lueur néon (post-traitement), halos additifs, vignettes, shaders GLSL. |
| `src/hud.rs` | Interface : panneaux, barres, bannière de vague, écrans titre, pause et fin. |

La simulation (`Game::step(dt, Input)`) ne dépend pas du dessin : les règles et l'équilibrage se testent sans fenêtre, avec un générateur aléatoire propre à chaque partie, donc reproductible.

### Tests et vérifications

```
cargo test
cargo clippy --all-targets
```

### Équilibrage

```
cargo test --release balance_report -- --ignored --nocapture
```

Joue 20 parties avec trois pilotes (joueur immobile, collecteur qui n'esquive rien, bot esquiveur) et affiche la vague atteinte, le score et la durée. À relancer après toute modification des réglages de `src/game.rs` (`BLAST`, `WALK`, `cfg()`, etc.).

### Captures d'écran

Le binaire contient un outil de test qui joue sans clavier, enregistre une capture et quitte :

```
cargo run --release -- <mode> [images] [vague] [graine] [nobloom] [got=0.6] [far]
```

- `mode` : `play` (pilote automatique), `idle` (joueur immobile), `pause`, `title` ou `over`.
- `images` : nombre d'images à jouer avant la capture (240 par défaut), `vague` : vague de départ, `graine` : graine du hasard.
- `nobloom` : sans lueur. `got=0.6` : 60 % du quota déjà ramassé (planète allumée). `far` : dangers placés à l'opposé du joueur, pour vérifier qu'on ne les voit pas à travers la planète.

La capture est écrite dans le dossier courant sous le nom `shot-<mode>-w<vague>-<images>.png` (ignoré par git), et la durée moyenne et maximale d'une image est affichée : avec la vsync à 60 Hz, elle doit rester proche de 16,6 ms.

### Notes techniques

- Rendu en mode immédiat avec macroquad 0.4. La planète est une cube-sphère de 6 × 8 × 8 dalles ; chaque dalle a un cadre, un panneau et, quand elle est allumée, un cœur lumineux.
- La lueur néon est un post-traitement : copie de l'écran, extraction des pixels brillants, flou séparable au quart de résolution, ajout par mélange additif. Si les shaders ne compilent pas sur une machine, le jeu continue sans lueur.
- Les effets lumineux sont dessinés avec un matériau additif qui écrit la profondeur, pour que la planète les masque. Les détails sont dans `CLAUDE.md`.
