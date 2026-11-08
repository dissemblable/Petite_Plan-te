# Petite Planète : consignes pour Claude

Jeu d'arcade 3D en Rust (macroquad 0.4 avec sa fonctionnalité `audio`, édition 2024, une seule dépendance). Un astronaute court sur une mini-planète en dalles cubiques, ramasse des cristaux et survit à des vagues de dangers. Les règles, les commandes du jeu et l'organisation des fichiers sont dans `README.md` ; ce fichier couvre ce qu'il faut savoir pour modifier le projet sans casser quoi que ce soit.

## Environnement

- Windows 11, PowerShell. `cargo` n'est pas dans le PATH du shell des outils : commencer chaque commande par `$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"`.
- Langue : français pour l'interface du jeu, les commentaires, les messages de commit et les docs. Jules écrit en français.
- Ne pas committer sans demande explicite. Ne jamais supprimer `record.txt` à la racine : c'est le record réel du joueur (« score vague », ignoré par git ; un ancien fichier avec le score seul doit rester lisible).
- Fins de ligne : les fichiers du dépôt sont en LF. Un script Python lancé sous Windows réécrit un fichier en CRLF s'il l'ouvre en mode texte : l'ouvrir en binaire, ou passer par l'outil d'édition.

## Vérifier un changement

1. `cargo test` (tout passe sans fenêtre), `cargo clippy --release --all-targets` et `cargo build --release` sans aucun warning.
2. Changement de règles ou de difficulté : `cargo test --release balance_report -- --ignored --nocapture`, puis ajuster si besoin les seuils des tests `standing_still_loses_quickly` et `dodging_matters_and_the_game_stays_hard` (ils sont volontairement larges).
3. Changement visuel : lancer l'outil de capture (voir `README.md`, section Captures d'écran) depuis un dossier temporaire, puis lire les PNG. Les captures et `record.txt` sont écrits dans le dossier courant. Vérifier aussi la durée d'image affichée (≈ 16,6 ms à 60 Hz) et regarder la capture `far` pour s'assurer que rien ne se voit à travers la planète.
4. Je ne peux pas jouer au clavier : le ressenti (difficulté, animations en mouvement) est à dire à Jules comme non vérifié.

## Architecture et règles de dépendance

- `sphere.rs` et `game.rs` ne dessinent jamais et n'appellent aucune fonction macroquad qui exige la fenêtre (les types mathématiques et `Color` sont permis). C'est ce qui rend la simulation testable sans fenêtre : tout reste appelable via `Game::step(dt, Input)`.
- Le hasard de partie vient de `Game.rng` (xorshift, déterministe). `macroquad::rand` est réservé aux effets purement visuels (tremblement de caméra).
- Les réglages de difficulté sont en tête de `game.rs` (constantes et `cfg(vague)`). Ne pas les disperser ailleurs.
- Nouvelle mécanique de jeu : l'ajouter dans `game.rs` avec un test de règle sans fenêtre, mettre à jour `Game::bot` si elle demande une réaction (il sert aux tests, aux simulations d'équilibrage et à l'outil de capture), puis l'afficher dans `models.rs` ou `draw.rs`, et l'expliquer dans la bannière de vague (`hud::wave_text`) si elle est liée à une vague.
- Sons : `game.rs` n'en joue aucun. Il pousse des événements (`Sfx`) dans `Game.sfx`, vidé au début de chaque pas, et `main.rs` les passe à `sound::Sounds::update`. Les sons sont synthétisés dans `sound.rs` puis emballés en WAV 16 bits mono à 44,1 kHz : le décodeur de quad-snd plante sur un fichier mal formé, d'où le test de l'en-tête.
- Palette : `HOTPINK` et non `PINK`, `ACID` et non `LIME` ou `GREEN`, qui existent déjà dans macroquad (conflit d'import avec les `use ...::*`).
- Les touches sont lues par position physique (scancode) : `KeyCode::W`/`A`/`D` sont Z/Q/D sur AZERTY. Ne pas ajouter d'alias Z/Q, ils se doubleraient.
- Pas de ressource externe (image, son, police) sans demande : tout est généré par le code.

## Pièges de rendu (macroquad / miniquad)

- **Profondeur.** miniquad n'active le test de profondeur que si le matériau a `depth_write: true`. Avec `false`, le matériau ignore la planète et ses effets se voient à travers elle. Le matériau additif (`fx::sprite_material`) écrit donc la profondeur ; en contrepartie, les décalques se dessinent du plus bas au plus haut et les halos (`Sprite`) du plus loin au plus proche, sinon ils se masquent entre eux.
- **Hauteurs au-dessus de la sphère de rayon `R`.** Dalle : cadre 0, panneau +0,03, joints +0,04, cœur +0,05. Marquages : zone d'impact et liseré de l'onde +0,065, traînée +0,07, aura +0,075, lignes +0,09, bandes d'onde +0,10 à +0,11, trait blanc de l'onde +0,12. Un nouveau marquage doit rester au-dessus de +0,05 et être dessiné dans l'ordre croissant.
- **Texte 2D.** Il ne passe pas par le test de profondeur : pour un point flottant ancré en 3D, tester `sphere::hidden_by_planet` avant de l'afficher.
- **Matériaux et uniformes.** Quand macroquad vide les dessins en attente (par exemple à `set_camera`), il recopie leurs anciens paramètres dans le matériau. Régler les uniformes après `set_camera`, juste avant de dessiner (voir `fx::Bloom::pass`), sinon un passage utilise les paramètres du précédent.
- **Lignes.** `draw_line_3d` ne fonctionne pas avec un matériau personnalisé (leur type de primitive est le triangle) : l'additif ne vaut que pour des maillages.
- **Textures de rendu.** La copie de l'écran (`grab_screen`) est à l'envers par rapport à la convention de macroquad : `flip_y: true` au premier passage seulement. Pour les passages vers une cible, `Camera2D` avec un `zoom` positif. La texture copiée doit avoir exactement la taille de l'écran (elle est recréée si la fenêtre change).
- **Alpha du tampon.** Les mélanges translucides baissent l'alpha du tampon d'écran, sans effet dans la fenêtre. L'outil de capture force l'alpha à 255, sinon les PNG s'affichent avec un voile gris.
- **Limites d'un dessin.** 10 000 sommets et 5 000 indices (`u16`) : un maillage plus gros doit être découpé (la planète en utilise trois).
- **Police.** La police par défaut gère les accents mais pas les glyphes exotiques (par exemple `·`) ; vérifier sur capture.

## Où modifier quoi

| Je veux… | Je touche… |
| --- | --- |
| Changer la difficulté, un danger, une vague | `game.rs` : constantes et `cfg()` en tête, puis les tests et le rapport d'équilibrage |
| Changer l'apparence d'un personnage ou d'un objet | `models.rs` |
| Changer la planète (dalles, allumage, assombrissement) | `planet.rs` |
| Changer la lueur, les halos, les vignettes | `fx.rs` (réglages en tête : seuil, intensité, passes de flou) |
| Changer l'interface ou les textes des écrans | `hud.rs` |
| Changer la scène, la caméra, les marquages au sol | `draw.rs` (la position de la caméra est `Game::cam_goal` dans `game.rs`) |
| Changer un modificateur de vague ou le boss | `game.rs` (`Twist`, `start_wave`, `step`, réglages en tête), textes dans `hud::wave_text`, rendu dans `planet.rs` (éclipse), `draw.rs` (traînée qui brûle) et `models.rs` (boss) |
| Changer un son | `sound.rs` (synthèse, volumes) ; l'événement qui le déclenche est un `Sfx` de `game.rs` |
