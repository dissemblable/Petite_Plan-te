# Petite Planète

Jeu d'arcade en 3D écrit en Rust avec [macroquad](https://macroquad.rs). Un astronaute court sur une mini-planète faite de dalles cubiques, ramasse des cristaux d'énergie et survit à des vagues de dangers de plus en plus variés : météorites, ondes de choc, drones chasseurs, salves en anneau, puis cristaux à ramasser dans l'ordre des couleurs.

Le style est néo-futuriste et cubique : planète dont les dalles s'allument au fil de la vague, lueur néon, interface à panneaux lumineux. Tout est généré par le code, sons compris : aucun fichier image, son ou police à installer, et une seule dépendance (`macroquad`, avec sa fonctionnalité `audio`).

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
| Demi-tour (en un quart de seconde) | `↓` ou `S` |
| Sauter | `Espace` |
| Dash | `Maj` |
| Pause | `P` ou `Échap` |
| Couper ou remettre la lueur | `G` |
| Quitter | `Échap` depuis la pause, l'écran titre ou l'écran de fin |
| Départ rapide (vague 3), une fois la vague 5 atteinte | `Entrée` sur l'écran titre ou l'écran de fin |

`Espace` lance aussi une partie depuis l'écran titre et la relance depuis l'écran de fin, après 1 s : un saut au moment de perdre ne relance pas la partie. Les touches sont lues par leur position physique : sur AZERTY, les touches `W` `A` `D` d'un clavier QWERTY sont `Z` `Q` `D`.

## Règles

**Les vagues.** Chaque vague demande de ramasser un quota de cristaux : `6 + 2 × numéro de vague`. La planète s'allume dalle par dalle à mesure que le quota se remplit. Chaque vague commence par une pause sans danger (2,5 s pour la première, 3,5 s ensuite), pendant laquelle une bannière présente son nouveau danger. À la fin d'une vague : +1 vie (5 au maximum) et un bonus de `100 × numéro de vague`, doublé si aucun coup n'a été reçu pendant la vague (« SANS FAUTE », affiché sous la barre de la vague tant qu'il tient). Chaque vague ajoute un danger aux précédentes :

| Vague | Nom | Nouveauté |
| --- | --- | --- |
| 1 | Pluie | Des météorites tombent. Leur zone d'impact apparaît au sol environ 2 s avant la chute. Être dans la zone, au sol, à l'impact coûte une vie. Sauter au bon moment esquive. |
| 2 | Ondes | Chaque impact lance une onde circulaire (10 unités/s) qu'il faut franchir en sautant : un saut au bon moment suffit toujours, même en la fuyant. À l'impact, l'onde blesse déjà jusqu'à 0,6 au-delà de la zone : un liseré autour de la zone le montre, et la zone passe au rouge dès qu'on est dans le liseré. Des cristaux dorés apparaissent au centre des zones d'impact : appât risqué. |
| 3 | Chasseurs | Un drone poursuit le joueur (moins vite que lui) ; un faisceau rouge le signale au-delà de l'horizon. Retomber dessus après un saut l'écrase. L'attirer sous une météorite le tue aussi. |
| 4 | Salves | Toutes les 10 à 16 s, six météorites tombent presque ensemble (en 0,125 s) en anneau autour du joueur. On peut sortir de l'anneau tout de suite, ou rester au centre : un seul saut, au moment où les six ondes s'y rejoignent, les franchit toutes. |
| 5 | Ordre | Les cristaux ont trois couleurs (6 sur la planète, 2 de chaque) et se ramassent dans l'ordre : cyan, rose, vert, puis on recommence. Un cristal de la mauvaise couleur retire une case du quota et casse le combo, même pendant un dash. On ne ramasse qu'au sol : un saut passe par-dessus un mauvais cristal. Le cristal doré est un joker. L'aura du joueur, le panneau ORDRE et les faisceaux (vifs pour la couleur attendue, pâles pour les autres) montrent la couleur attendue. Rien d'autre ne durcit : toujours un seul drone. |
| 6 et plus | Chaos | Tout à la fois, avec un deuxième drone, puis d'autres. |
| 7 et plus | Modificateurs | Chaque vague, sauf celles de boss, tire au hasard un modificateur, annoncé par la bannière et jamais le même deux fois de suite. **Dalles fragiles** : la traînée du joueur brûle pendant 3 s (sauf sous ses pieds), il ne faut pas repasser dessus. **Gravité faible** : sauts presque deux fois plus hauts et plus longs. **Éclipse** : la face cachée du soleil s'éteint, seuls les marquages au sol restent visibles. **Cristaux fuyants** : les cristaux à moins de 6 unités s'éloignent du joueur, moins vite que lui. |
| 10, 15, 20… | Boss | Un drone géant, plus lent que les autres, qu'on ne peut pas écraser : on rebondit dessus. Chaque météorite qui le touche l'étourdit 1 s ; la troisième le détruit et gagne la vague. Pas de modificateur pendant une vague de boss. |

Au fil des vagues, les météorites tombent plus souvent (intervalle `max(0,7 ; 2,2 − 0,2 × vague)` secondes) et il y a plus de drones : 1 des vagues 3 à 5, 2 à la vague 6, 3 aux vagues 7 et 8, 4 à partir de la vague 9. Dès la vague 2, une part des météorites vise l'endroit où le joueur sera à l'impact s'il continue tout droit : 15 % à la vague 2, puis 15 % de plus par vague, jusqu'à 50 % à partir de la vague 5.

**Score et combo.** Un cristal vaut 10 points, un cristal doré 50 (et compte pour 3 dans le quota), un drone détruit 30, qu'il soit écrasé ou tué par une météorite, le boss 100 par météorite reçue et 300 à la dernière. Ramasser un cristal ou détruire un drone dans les 3 s qui suivent le précédent allonge la chaîne. Le multiplicateur gagne un palier tous les 4 maillons : ×1, ×2 à 4 maillons, et ainsi de suite jusqu'à ×5 à 16 maillons. Un coup ou un cristal de la mauvaise couleur remet la chaîne à zéro. Les cristaux disparaissent au bout de 11 s ; le doré disparaît avec l'impact de sa météorite.

**Vies et gestes.** On commence avec 3 vies. Un coup donne 1,5 s d'invulnérabilité. Le dash (0,2 s, rechargé en 1,4 s) rend invulnérable pendant qu'il dure ; les réacteurs de l'astronaute s'allument quand il est prêt. Un saut demandé juste avant l'atterrissage est mémorisé.

**Fin de partie.** L'écran de fin dit ce qui a donné le dernier coup et comment l'éviter, puis affiche la plus longue chaîne, les drones détruits et les cristaux dorés ramassés.

**Son.** Les sons sont générés par le code : ramassage (la note monte avec la chaîne), mauvais cristal, impact, coup reçu, dash, vague terminée, drone qui s'arme, et un double bip quand une onde arrive dans le dos.

**Record.** Le meilleur score et la plus haute vague atteinte sont enregistrés dans `record.txt` (« score vague »), dans le dossier d'où on lance le jeu. Un ancien fichier, avec le score seul, reste lisible. La vague 5 atteinte une fois débloque le départ rapide à la vague 3.

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
| `src/sound.rs` | Sons générés par le code (synthèse, WAV en mémoire), joués d'après les événements de la simulation. |

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

Joue 20 parties avec trois pilotes (joueur immobile, collecteur qui n'esquive rien, bot esquiveur) et affiche la vague atteinte, le score, la durée, la plus longue chaîne et ce qui a donné le dernier coup. À relancer après toute modification des réglages de `src/game.rs` (`BLAST`, `WALK`, `cfg()`, etc.).

### Captures d'écran

Le binaire contient un outil de test qui joue sans clavier, enregistre une capture et quitte :

```
cargo run --release -- <mode> [images] [vague] [graine] [nobloom] [got=0.6] [far]
```

- `mode` : `play` (pilote automatique), `idle` (joueur immobile), `pause`, `title` ou `over`.
- `images` : nombre d'images à jouer avant la capture (240 par défaut), `vague` : vague de départ, `graine` : graine du hasard.
- `nobloom` : sans lueur. `got=0.6` : 60 % du quota déjà ramassé (planète allumée). `far` : dangers placés à l'opposé du joueur, pour vérifier qu'on ne les voit pas à travers la planète.

La capture est écrite dans le dossier courant sous le nom `shot-<mode>-w<vague>-<images>.png` (ignoré par git), et la durée moyenne et maximale d'une image est affichée : avec la vsync à 60 Hz, elle doit rester proche de 16,6 ms. L'outil indique aussi si les sons se sont chargés (il ne les joue pas).

### Notes techniques

- Rendu en mode immédiat avec macroquad 0.4. La planète est une cube-sphère de 6 × 8 × 8 dalles ; chaque dalle a un cadre, un panneau et, quand elle est allumée, un cœur lumineux.
- La lueur néon est un post-traitement : copie de l'écran, extraction des pixels brillants, flou séparable au quart de résolution, ajout par mélange additif. Si les shaders ne compilent pas sur une machine, le jeu continue sans lueur.
- Les effets lumineux sont dessinés avec un matériau additif qui écrit la profondeur, pour que la planète les masque. Les détails sont dans `CLAUDE.md`.
