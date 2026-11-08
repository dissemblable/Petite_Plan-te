# Petite Planète : analyse de game design

Analyse du jeu tel qu'il est au commit `a6325da` (vague 5 « Ordre » comprise), le 7 novembre 2026.

> **Suivi (7 novembre 2026).** Priorités 1 et 2 appliquées. Écarts avec ce document : ondes à 10 u/s et non 4 (avec une onde lente, une course en biais laisse l'onde rattraper le joueur sans qu'un saut suffise) ; ramassage seulement sous 0,5 de hauteur et non 1,0 (avec 1,0, un saut ne passe pas par-dessus un cristal pris en son centre) ; pause de la vague 1 portée à 2,5 s pour que la bannière reste lisible ; caméra plus plongeante, mais pas plus haute (plus haute, elle rapetisse la planète). Priorité 3 appliquée le 8 novembre : boss aux vagues 10, 15… et non 5 (la vague 5 introduit déjà l'ordre), avec 3 points de vie (avec un seul, il suit le joueur de si près que le bot gagnait la vague de boss en 10 s), et pas de modificateur pendant une vague de boss.

## En bref

- **La base est solide.** Une nouveauté par vague, des dangers annoncés longtemps à l'avance, une planète qui s'allume au fil du quota, des parties courtes : le jeu donne envie de relancer.
- **Le vrai danger n'est pas la météorite, c'est l'onde.** Les ondes causent environ 75 % des coups reçus. Les météorites elles-mêmes ne touchent presque jamais un joueur qui les regarde (2 % des coups).
- **Deux règles cachées rendent les ondes injustes.** Une onde qui arrive dans le dos ne peut pas être sautée si l'on continue de courir. Au centre d'une salve, le coup est garanti, alors que la bannière conseille justement d'y rester.
- **La vague 5 est un mur.** Elle dure deux fois plus longtemps que la vague 4. Le bot esquiveur y arrive à chaque partie, mais la moitié de ses parties s'y arrêtent.
- **Le combo est toujours au maximum (×5)**, même pour un pilote qui joue mal : il ne récompense pas l'adresse.
- **Il manque du son**, un retour sur la cause de la mort, et un repère pour les menaces hors champ (drones, dangers dans le dos).
- Les corrections prioritaires sont presque toutes des changements de quelques lignes (section « Pistes d'amélioration »).

## Méthode et limites

1. Lecture complète du code (`game.rs`, `main.rs`, `hud.rs`, `draw.rs`, `models.rs`, `planet.rs`, `sphere.rs`) et du `README.md`.
2. Rapport d'équilibrage existant (`balance_report`, 20 parties par pilote).
3. **200 parties instrumentées** dans une copie temporaire du jeu (hors du dépôt), qui note la cause de chaque coup, la durée des vagues, l'origine des points et la longueur des combos. Deux pilotes : le **naïf** (ramasse le cristal le plus proche, n'esquive rien) et l'**esquiveur** (`Game::bot`).
4. **Expériences ciblées** : salve avec un joueur au centre (toutes les façons de sauter et de dasher), onde qui arrive dans le dos (en courant ou à l'arrêt).
5. Quatre captures d'écran (vagues 1, 2, 3 et 5).

**Limites.** Un bot n'est pas un joueur humain : il réagit sans délai, mais il ne sait pas gérer les salves ni anticiper les drones. Les chiffres montrent où le jeu est dur ou injuste, pas ce qu'on ressent en jouant. Rien n'a été testé au clavier : la difficulté ressentie, la maniabilité et le plaisir restent à vérifier en jouant.

## Ce qui marche

1. **Une vague, une nouveauté.** Pluie, Ondes, Chasseurs, Salves, Ordre : chaque vague ajoute un seul danger, nommé et expliqué par la bannière. C'est la bonne façon d'apprendre un jeu sans tutoriel.
2. **Des dangers bien annoncés.** Une zone d'impact apparaît environ 2,2 s avant la chute. Un anneau se remplit jusqu'à l'impact, la zone passe au rouge quand on est dedans, et les dalles s'assombrissent sous le danger. Résultat mesuré : le bot esquiveur n'est presque jamais touché par une météorite (13 coups sur 692).
3. **Une progression visible dans le décor.** Les dalles s'allument une à une à mesure que le quota se remplit. On lit l'avancement sans regarder l'interface, et la planète entièrement allumée est une vraie récompense.
4. **Des choix risqués qui rapportent.** Le cristal doré au centre d'une zone d'impact, l'écrasement des drones, le bonus doublé sans faute : le jeu propose régulièrement de prendre un risque pour gagner plus.
5. **Un jeu qui pardonne au bon endroit.** Un saut demandé un peu tôt est mémorisé, un coup donne 1,5 s d'invulnérabilité, le rayon d'écrasement d'un drone (1,5) est plus grand que son rayon de contact (0,9), et chaque vague rend une vie.
6. **Accessibilité.** Chaque couleur de cristal a sa propre forme (prisme, cube, losange), et les touches sont lues par position physique, ce qui fait marcher l'AZERTY sans réglage.
7. **Des parties courtes.** Environ 2 minutes pour un bon joueur, une relance immédiate et un record enregistré : la boucle « encore une » fonctionne.
8. **Un outil d'équilibrage rare pour un projet de cette taille.** La simulation déterministe, le bot et le rapport d'équilibrage permettent de régler le jeu sur des chiffres.
9. **Une identité visuelle forte et une image fluide** (16,6 ms par image dans toutes les captures).

## Ce qui ne va pas

Les problèmes sont classés du plus grave au moins grave.

### 1. Les ondes : la menace principale, avec des règles invisibles (grave)

**Constat.** Sur 100 parties de l'esquiveur, 516 coups sur 692 (75 %) viennent d'ondes, et 79 morts sur 100. Pour le naïf, c'est 73 % des coups. Le jeu qui s'annonce comme une « pluie de météorites » est en réalité un jeu où il faut sauter des ondes.

Ce n'est pas un problème en soi. Le problème, ce sont deux règles que le joueur ne peut pas deviner.

**a) L'onde qui arrive dans le dos.** L'onde avance à 7 unités/s, le joueur court à 6,5. Si l'on vient de sortir d'une zone et qu'on continue à fuir, l'onde nous rattrape à 0,5 unité/s : il lui faut jusqu'à 2,4 s pour nous dépasser, alors qu'un saut ne protège que 0,59 s. Mesure : un joueur à 3,2 ou 3,5 unités de l'impact qui continue de courir est touché quel que soit le moment où il saute (0 instant de saut sur 120 l'évite). S'il s'arrête, environ 26 instants sur 120 marchent, soit une fenêtre d'un peu moins d'une demi-seconde, ce qui est juste.

Le bon réflexe (s'arrêter, se retourner, sauter) va contre l'instinct (fuir l'explosion), et rien ne l'enseigne. Le bot le sait (`ring_behind` dans `Game::bot`) ; le joueur, lui, ne l'apprend qu'en perdant des vies.

**La règle à respecter.** Pour qu'un saut puisse toujours franchir l'onde, il faut que l'onde traverse le joueur en moins de 0,59 s. L'anneau qui blesse fait 1,2 de large, il faut donc un écart de vitesse d'au moins 2 unités/s entre l'onde et le joueur : `SHOCK_SPEED` ≤ 4,4 ou ≥ 8,6. La valeur actuelle, 7, est dans la zone interdite.

**b) La zone dessinée est plus petite que la zone dangereuse.** À l'impact, l'onde naît au bord de la zone (rayon 3) et blesse déjà de 2,4 à 3,6. Un joueur qui croit être sorti à temps (entre 3 et 3,6) est touché à l'instant même de l'impact s'il ne saute pas. Le bot, lui, considère qu'il n'est en sécurité qu'au-delà de 4,6 (`BLAST + 1.6` dans `Game::bot`).

### 2. Salves : le conseil « reste au centre » est un piège (grave)

**Constat.** La bannière de la vague 4 dit : « reste au centre ou fuis par une brèche ». Or, depuis la vague 2, chaque impact lance une onde. Les six ondes de la salve se rejoignent au centre, mais décalées dans le temps (une chute toutes les 0,125 s) : elles balaient le centre pendant 0,8 s d'affilée, plus que la durée d'un saut.

Mesures, joueur immobile au centre :

| Réaction | Résultat |
| --- | --- |
| Rien | touché à chaque fois |
| Un saut, à n'importe quel instant (240 essais) | touché à chaque fois |
| Sauts enchaînés sans arrêt | touché à chaque fois |
| Un saut et un dash, toutes combinaisons (14 400 essais) | touché à chaque fois |

À l'inverse, **courir tout droit, dans n'importe quelle direction**, suffit, à condition de partir vite :

| Départ après | Touché |
| --- | --- |
| 0 à 0,3 s | 0 % |
| 0,6 s | 20 % |
| 0,9 s | 68 % |
| 1,2 s | 100 % |

Donc le centre n'est jamais sûr, la brèche ne sert à rien, et la salve ne teste que le temps de réaction. Le joueur qui suit le conseil affiché perd une vie à chaque salve.

### 3. La vague 5 est un mur (important)

| Vague | Durée moyenne (esquiveur, pause comprise) | Parties de l'esquiveur qui l'atteignent | Parties du naïf qui l'atteignent |
| --- | --- | --- | --- |
| 1 | 12 s | 100 % | 100 % |
| 2 | 15 s | 100 % | 100 % |
| 3 | 17 s | 100 % | 100 % |
| 4 | 22 s | 100 % | 85 % |
| 5 | 40 s | 100 % | 46 % |
| 6 | 42 s | 50 % | 0 % |
| 7 | – | 6 % | 0 % |

La vague 5 introduit la règle la plus exigeante du jeu (l'ordre des couleurs) **en même temps** qu'un deuxième drone, des météorites plus fréquentes (toutes les 1,2 s), la part maximale de météorites qui visent devant le joueur (50 %) et un quota de 16. Elle dure presque deux fois plus que la vague 4.

Deux détails la rendent plus dure qu'elle n'en a l'air :

- **On ne peut pas sauter par-dessus un mauvais cristal.** Le ramassage marche jusqu'à 1,8 de hauteur, et un saut monte à 1,45. Le dash ramasse aussi. Les deux tiers des cristaux deviennent des obstacles qu'il faut contourner avec des commandes de char (rayon de virage d'environ 2,5).
- **Rien ne distingue de loin les bons cristaux des mauvais.** Les faisceaux verticaux, qui servent à repérer les cristaux au-delà de l'horizon, ont tous la même intensité.

### 4. Le combo est toujours au maximum (important)

Le multiplicateur atteint ×5 au cinquième ramassage enchaîné, et la fenêtre pour enchaîner est de 3 s alors qu'un cristal est en général à moins de 2 s de marche. Mesure : dans 100 % des parties, des deux pilotes (y compris celui qui n'esquive rien), la chaîne atteint au moins 7 maillons ; dans la plupart, elle atteint 13 ou plus.

Le combo n'est donc pas une récompense d'adresse : c'est l'état normal, qu'on perd seulement en prenant un coup. Le score revient presque à multiplier par 5 le nombre de cristaux.

### 5. Les menaces hors champ (important)

- **Les drones** apparaissent à 12 à 22 unités, hors de l'écran, sans aucun repère. Ils causent 24 % des coups de l'esquiveur. Les cristaux ont un faisceau visible au-delà de l'horizon, pas les drones.
- **La caméra montre peu de terrain.** Sur les captures, environ 40 % de l'écran est du ciel, et la zone derrière l'astronaute est coupée par le bas de l'écran. Or les deux causes principales de coups (ondes et drones) arrivent souvent par derrière.

### 6. Commandes et ergonomie (important, facile à corriger)

- **`S` ne fait rien**, alors que l'écran titre annonce « ZQSD/WASD : bouger ». Le joueur qui veut reculer appuie sur `S` et rien ne se passe.
- **`Échap` quitte le jeu immédiatement**, en pleine partie et sans confirmation. Beaucoup de joueurs appuient sur `Échap` pour mettre en pause : ils perdent leur partie.
- **`Espace` relance la partie dès l'écran de fin.** Comme `Espace` sert aussi à sauter, un joueur qui saute au moment de mourir relance aussitôt et ne voit ni son score ni son éventuel record.

### 7. Aucun son (important)

Le jeu est muet. Le son fait une grande partie de la sensation de jeu (ramassage, impact, coup reçu), et c'est aussi un moyen d'annoncer ce qu'on ne voit pas : une onde qui approche dans le dos, un drone qui s'arme hors champ.

### 8. Une fin de partie qui n'apprend rien (moyen)

L'écran de fin affiche « PERDU ! », la vague et le score. Il ne dit pas ce qui a tué le joueur. Vu les règles cachées des ondes (point 1), c'est justement l'information qui permettrait de progresser. Il ne montre pas non plus de statistiques (meilleur combo, drones écrasés, cristaux dorés) qui donneraient des objectifs pour la partie suivante.

### 9. Après la vague 5, plus rien de neuf (moyen)

À partir de la vague 6 (« Chaos »), seuls la fréquence des météorites (jusqu'à la vague 8) et le nombre de drones (vagues 7 et 9) augmentent. Aucune nouvelle décision n'apparaît. Comme le bot dépasse rarement la vague 7, ce n'est pas urgent, mais un bon joueur humain trouvera les vagues 6 à 10 répétitives.

### 10. Textes et règles qui ne correspondent pas au code (mineur)

- **Bannière de la vague 6** : « Les météorites visent devant toi ». En réalité, une partie des météorites visent devant le joueur dès la vague 2 (15 %), et cette part plafonne à 50 % dès la vague 5. Rien ne change à la vague 6. Le `README.md` dit aussi que la part augmente à partir de la vague 6.
- **Cristal doré, « 7 s »** : la météorite qui l'accompagne tombe 2,2 s après son apparition et le détruit. Les 7 s ne servent jamais, et le doré ne clignote jamais avant de disparaître.
- **Drone écrasé par une météorite** : 20 points et aucun maillon de combo, contre 30 × le multiplicateur (jusqu'à 150) pour un drone sur lequel on retombe. Attirer un drone sous une météorite est plus difficile et plus risqué, mais rapporte jusqu'à 7 fois moins. Mesure : 1 % des points de l'esquiveur.

### 11. Lisibilité de l'interface (mineur)

- **Le conseil de la bannière n'a pas de fond.** Il passe sur la planète et les objets, et se lit mal (capture de la vague 2).
- **À la vague 1, la bannière dure 4 s mais la pause 1,5 s** : elle couvre le centre de l'écran pendant 2,5 s de jeu réel.
- **Le bonus « sans faute » n'est pas affiché pendant la vague.** Le joueur ne sait pas ce qu'il est en train de risquer.
- **La recharge du dash est en haut à gauche**, loin du regard. Pendant l'action, on regarde l'astronaute.

## Chiffres détaillés

**Origine des coups reçus (100 parties par pilote)**

| Cause | Esquiveur | Naïf |
| --- | --- | --- |
| Ondes (dont ondes de salve) | 516 (75 %), dont 179 de salve | 455 (73 %) |
| Drones | 163 (24 %) | 83 (13 %) |
| Météorites (dont salves) | 13 (2 %) | 89 (14 %) |
| **Coups qui ont tué** | ondes 79, drones 20, météorite 1 | ondes 76, drones 13, météorites 11 |

Le bot esquiveur ignore complètement les salves : les 179 coups d'onde de salve surestiment sans doute ce que subirait un humain attentif.

**Origine des points (moyenne par partie)**

| Source | Esquiveur | Naïf |
| --- | --- | --- |
| Cristaux | 2 161 (45 %) | 1 225 (45 %) |
| Bonus de fin de vague | 1 808 (37 %) | 926 (34 %) |
| Drones écrasés | 432 (9 %) | 0 |
| Cristaux dorés | 304 (6 %) | 513 (19 %) |
| Drones tués par une météorite | 54 (1 %) | 15 (0 %) |

**Cristaux dorés** : le naïf en ramasse 68 %, l'esquiveur 9 % (il évite les cristaux condamnés). Le doré est très rentable pour qui ose le prendre, et le naïf montre que le risque immédiat (la météorite) est faible : le vrai risque, c'est l'onde qui suit.

**Mauvaises couleurs ramassées (vague 5 et plus)** : 3,7 par partie pour le naïf, 1,5 pour l'esquiveur.

## Pistes d'amélioration

Les pistes sont classées par priorité. L'effort est estimé pour ce code : « petit » veut dire quelques lignes.

### Priorité 1 : corriger ce qui est injuste (petit effort, gros effet)

| # | Piste | Où | Effort |
| --- | --- | --- | --- |
| 1 | **Sortir `SHOCK_SPEED` de la zone interdite.** Deux options. Lente (≈ 4) : fuir marche, ce qui récompense l'instinct, et l'onde reste visible plus longtemps ; mais les ondes s'accumulent davantage à l'écran en fin de partie. Rapide (≈ 10) : l'onde passe toujours vite et la seule réponse est « sauter au bon moment ». Je recommande d'essayer la lente d'abord. | `game.rs`, constante `SHOCK_SPEED` | petit |
| 2 | **Montrer la vraie zone dangereuse à l'impact** : à partir de la vague 2, dessiner un liseré jusqu'à `BLAST + 0,6`, pour que « je suis sorti de la zone » veuille dire « je suis à l'abri à l'impact ». | `draw.rs`, marquages des météorites | petit |
| 3 | **Rendre le centre de la salve vraiment sûr**, au choix : faire tomber les six météorites presque ensemble (écart d'altitude de 0,3 au lieu de 1,5) pour qu'un seul saut bien placé franchisse les six ondes à la fois ; ou ne pas lancer d'onde pour les météorites de salve. La première option crée un beau moment de jeu (« le saut parfait »). Si l'on veut aussi que la brèche serve, il faut rapprocher l'anneau pour que les zones se chevauchent partout sauf à la brèche. | `game.rs`, `salvo()` | petit |
| 4 | **Corriger les commandes** : `Échap` met en pause pendant le jeu (et quitte depuis la pause ou l'écran titre) ; `Espace` n'est accepté à l'écran de fin qu'après environ 1 s ; `S` fait un demi-tour rapide, ce qui aide aussi contre les ondes dans le dos et les météorites qui visent devant (ou, au minimum, retirer `S` du texte de l'écran titre). | `main.rs`, `game.rs`, `hud.rs` | petit |
| 5 | **Corriger les textes** : conseil de la salve (selon la piste 3), conseil de la vague 6, et description de la visée dans le `README.md`. | `hud.rs`, `README.md` | petit |

### Priorité 2 : rendre le jeu plus lisible et plus juste dans la durée

| # | Piste | Où | Effort |
| --- | --- | --- | --- |
| 6 | **Combo qui demande de l'adresse.** Par exemple, un palier tous les 4 maillons : `mult = (1 + chain / 4).min(5)`. Le ×5 demande alors 16 ramassages enchaînés sans coup : le naïf y arriverait au moins une fois dans 36 % de ses parties, l'esquiveur dans 70 %. Autre option : raccourcir `CHAIN_TIME` à 2 s. | `game.rs`, `mult()` | petit |
| 7 | **Adoucir la vague 5.** Principe : la vague qui introduit une règle ne durcit rien d'autre. Garder 1 drone à la vague 5 (le deuxième arrive à la 6), et permettre de sauter par-dessus un cristal (ramassage seulement si la hauteur est sous 1,0). Le saut gagne au passage un nouvel usage. | `game.rs`, `cfg()` et ramassage | petit |
| 8 | **Distinguer les bons cristaux de loin** (vague 5 et plus) : faisceau vif pour la couleur attendue, faisceau pâle pour les autres. | `models.rs`, `crystal()` | petit |
| 9 | **Repérer les drones hors champ** : un faisceau vertical rouge comme celui des cristaux, ou une flèche au bord de l'écran. | `models.rs`, `drone()` | petit |
| 10 | **Écran de fin qui explique** : cause du dernier coup (« Touché par une onde »), avec un conseil adapté (« Une onde te suit ? Arrête-toi et saute »), plus quelques statistiques (meilleur combo, drones écrasés, cristaux dorés). | `game.rs` (noter la cause), `hud.rs` | moyen |
| 11 | **Du son généré par le code.** macroquad sait jouer un son à partir d'octets en mémoire (à vérifier : fonctionnalité audio de macroquad 0.4). Priorités : ramassage avec une note qui monte avec le combo, mauvais cristal, impact, onde qui approche dans le dos, drone qui s'arme, coup reçu, dash, vague terminée. | nouveau module de sons | moyen |
| 12 | **Interface** : fond derrière le conseil de la bannière, bannière limitée à la pause de début de vague, indicateur « SANS FAUTE » qui s'éteint au premier coup, recharge du dash visible sur l'astronaute (par exemple, les réacteurs qui s'allument quand le dash est prêt). | `hud.rs`, `models.rs` | petit à moyen |
| 13 | **Caméra** : essayer une caméra un peu plus haute et plus plongeante pour montrer plus de terrain, surtout derrière l'astronaute. À juger sur captures puis en jouant. | `game.rs`, `cam_goal()` | petit, à tester |
| 14 | **Récompenser le leurre** : un drone tué par une météorite compte comme un maillon de combo et vaut au moins autant qu'un drone écrasé. | `game.rs`, `impact()` | petit |

### Priorité 3 : du contenu pour les bons joueurs

À ne faire qu'une fois les priorités 1 et 2 réglées et vérifiées en jouant.

- **Un modificateur par vague à partir de la vague 7**, tiré au hasard et annoncé par la bannière. Quelques idées qui utilisent ce que le jeu a déjà :
  - **Dalles fragiles** : la traînée du joueur devient dangereuse quelques secondes. Le jeu devient un peu un serpent sur une sphère : il faut éviter ses propres traces.
  - **Gravité faible** : sauts plus hauts et plus longs, donc ondes faciles mais écrasements de drones plus difficiles à placer.
  - **Éclipse** : la face cachée du soleil s'assombrit, seuls les marquages restent lisibles.
  - **Cristaux fuyants** : les cristaux s'éloignent lentement du joueur.
- **Un boss toutes les 5 vagues**, par exemple un gros drone qui ne meurt que sous une météorite, ce qui donne enfin un rôle central au leurre.
- **Départ rapide** : les vagues 1 à 3 ne demandent presque pas d'esquiver (le pilote qui n'esquive rien passe les vagues 1 et 2 dans toutes ses parties et la vague 3 dans 85 % d'entre elles, en 33 s au total). Pour un joueur qui relance souvent, on pourrait proposer de commencer à la vague 3 une fois la vague 5 atteinte. À confirmer en jouant : un début calme peut aussi être un plaisir.

## Vérifier après chaque changement

1. `cargo test`, `cargo clippy --release --all-targets`, `cargo build --release`, sans warning.
2. `cargo test --release balance_report -- --ignored --nocapture`, puis ajuster si besoin les seuils de `standing_still_loses_quickly` et `dodging_matters_and_the_game_stays_hard`.
3. **Mettre à jour `Game::bot`** quand une règle change. Aujourd'hui il ignore les salves : le rapport d'équilibrage sous-estime donc le niveau qu'un humain attentif peut atteindre à partir de la vague 4.
4. Captures d'écran pour tout changement visuel (voir `README.md`).
5. **Ce que seul un humain peut juger, en jouant** :
   - Les ondes paraissent-elles justes ? Sait-on toujours pourquoi on a été touché ?
   - La salve se lit-elle à temps (moins de 0,3 s pour réagir aujourd'hui) ?
   - La vague 5 est-elle claire : sait-on quel cristal prendre sans regarder le panneau ORDRE ?
   - Les commandes de char (avancer et tourner, sans recul) sont-elles agréables avec une caméra qui tourne avec l'astronaute ?
   - Le combo donne-t-il envie de prendre des risques ?
