# Arklandia Engine

Dépôt **privé** du moteur de serveur Minecraft Java d'Arklandia. Arklandia Engine est un **fork de Pumpkin** (https://github.com/Pumpkin-MC/Pumpkin, base `204a94e`) : un serveur Minecraft écrit en Rust, modifié pour les besoins des serveurs Arklandia. Le nom « Pumpkin » appartient au projet d'origine ; ce dépôt n'est ni affilié à lui ni soutenu par ses auteurs (voir `NOTICE.md`).

- Version : **0.1.0** (pré-version), build `0.1.0+mc26.3` : Minecraft Java 26.3, protocole 777.
- Licence : **GNU GPL v3** (`LICENSE`), droits d'auteur d'origine conservés (`NOTICE.md`).
- Binaire : `arklandia-engine`, publié sous la forme `arklandia-engine-<version>-linux-x86_64-musl`.

## Licence et distribution

Le dépôt est **privé**. La GPL v3 n'oblige à publier le code source que si un binaire est **distribué à un tiers** ; ce n'est pas prévu. Si un binaire est un jour remis à un tiers (y compris sous forme de Release publique ou de copie sur un serveur qui n'est pas le nôtre), il doit être accompagné du **code source complet correspondant, sous GPL v3**, et ne perdre aucune mention d'attribution d'origine. Les plugins Ark sont des programmes distincts, dans un dépôt séparé : ils communiquent avec le moteur par l'interface de plugins (`pumpkin-plugin-api` / `pumpkin-plugin-wit`, sous MIT OR Apache-2.0) et ne font pas partie de ce dépôt.

## Ce que le moteur ajoute à Pumpkin

Le détail daté est dans `CHANGES.md` ; la configuration est décrite dans `docs/CONFIGURATION.md`.

- Fichier `pumpkin.toml` complété automatiquement (clés manquantes ajoutées sans toucher aux commentaires ni aux valeurs, sauvegarde `pumpkin.toml.bak-…`), jamais de plantage sur une clé inconnue.
- Bedrock désactivable, télémétrie remplacée par une section `[analytics]` inactive, section `[operators]` pour supprimer les opérateurs.
- Permissions de plugins accordées d'après la configuration (aucune question en console), journal de chargement des plugins groupé, compilation parallèle des plugins.
- Liens du serveur (`/bug`, `/discord`, `/site`, annonce à la connexion) et interface `server-links` pour les plugins.
- Interfaces de plugins facultatives : `event-extra` (contexte supplémentaire des événements, utilisé par ArkGuard), `server-links`, `blocks`, `database`. Les plugins qui ne les importent pas continuent de se charger.
- Remplacement d'une commande native par une commande de plugin, `[commands.overrides.<nom>]` pour désactiver ou restreindre une commande.
- Messages d'arrivée, de départ, de première connexion et de bienvenue personnalisables (`[messages]`).
- Messages privés `/msg` `/tell` `/w` `/whisper` `/m` `/r` `/reply` personnalisables (`[private_messages]`) avec un hook IPC pour un plugin de modération (espionnage, mute, vanish).
- Édition de blocs en masse sur plusieurs cœurs (pool dédié `ArkEdit`, `world_edit_threads`) et allocateur mimalloc sur musl.
- Correctif du chat pour les commandes en `//` (`//wand`, `//pos1`…), commande `/arkbug` (version et correctifs intégrés), et une trentaine de correctifs de comportement (lapins, mobs ignorant les joueurs en créatif, aquatiques, blocs, entités, perte de chunks, etc.).

## Compatibilité avec les plugins

Les noms internes de Pumpkin sont **volontairement conservés** pour que les plugins déjà compilés se chargent sans recompilation : crates `pumpkin`, `pumpkin-*`, paquet WIT `pumpkin:plugin`, API `pumpkin_plugin_api`, permissions et espaces de noms `pumpkin:…`, fichier de configuration **`pumpkin.toml`**, dossiers de données (`plugins/`, `world/`…) et expéditeur IPC `"Pumpkin"` du hook de messages privés. Seule l'identité visible est renommée (binaire, bannière, marque réseau `minecraft:brand`, journaux, `/arkbug`, fichiers de release).

Plugins Ark compatibles (0.1.0, dépôt `Arklandia-Plugins`) : ArkHome, ArkTPA, ArkPerm, ArkStaff, ArkWorld, ArkGuard, ArkSpawn. ArkStaff dépend du remplacement de commandes natives, de `server-links` et du hook de messages privés ; ArkGuard de `event-extra` ; ArkWorld de l'édition de blocs en masse.

## Compiler

Prérequis :

- Rust (`rust-toolchain.toml` : canal `stable`, `rust-version = 1.96`), avec la cible musl : `rustup target add x86_64-unknown-linux-musl` ; un compilateur C (gcc) et `musl-tools` pour les dépendances en C ;
- `ulimit -s unlimited` et `RUST_MIN_STACK=536870912` : le crate `pumpkin-data` (code généré, très volumineux) dépasse sinon la pile du compilateur ;
- beaucoup de mémoire et de temps : environ **40 minutes** pour une compilation complète en release (LTO, `codegen-units = 1`), nettement moins ensuite pour les modifications de `pumpkin`.

```sh
scripts/build.sh                 # -> target/x86_64-unknown-linux-musl/release/arklandia-engine
scripts/build.sh test            # tests rapides (markup, fusion de config, dispatcher, Anvil)
scripts/make-release.sh          # -> release/arklandia-engine-0.1.0/
```

Équivalent manuel :

```sh
ulimit -s unlimited
export RUST_MIN_STACK=536870912
cargo build --release --target x86_64-unknown-linux-musl
```

`CARGO_TARGET_DIR` choisit le dossier de compilation. `CARGO_NET_OFFLINE=true` évite le réseau si les dépendances sont en cache. `vendor/rustyline` est une copie patchée de `rustyline`, nécessaire à la compilation (exclue du workspace, référencée par `[patch]`).

## Démarrer

```sh
mkdir -p /srv/arklandia && cd /srv/arklandia
cp .../arklandia-engine-0.1.0-linux-x86_64-musl ./arklandia-engine && chmod +x arklandia-engine
./arklandia-engine
```

Au premier lancement, `pumpkin.toml` (même nom qu'avec Pumpkin) et les dossiers `world/` et `plugins/` sont créés dans le dossier courant. La bannière indique `Starting Arklandia Engine 0.1.0+mc26.3 (basé sur Pumpkin) Java Minecraft (Protocol 777)`. Les plugins `.wasm` se déposent dans `plugins/` ; leurs `allowed_permissions` se déclarent dans `pumpkin.toml` (section `[plugins]`).

## Configuration

Tout se règle dans `pumpkin.toml` (exemple complet et commenté : `docs/pumpkin.toml.example`). Les sections propres à Arklandia sont décrites dans `docs/CONFIGURATION.md` : `[operators]`, `[analytics]`, `[server_links]`, `[messages]`, `[private_messages]`, `[plugins]` (`world_edit_threads`), `[commands.overrides.<nom>]`, et le protocole IPC `spy_plugin`.

## Structure

```
crates/          moteur et bibliothèques (pumpkin, pumpkin-*), interface de plugins (pumpkin-plugin-api, pumpkin-plugin-wit)
tools/           pumpkin-codegen (génération des données), pumpkin-fuzzer
assets/          données de jeu nécessaires à la compilation et aux tests
vendor/          rustyline patché
docs/            CONFIGURATION.md, RELEASES.md, pumpkin.toml.example
scripts/         build.sh, make-release.sh
CHANGES.md       modifications d'Arklandia par rapport à Pumpkin (datées)
NOTICE.md        attributions
MIGRATIONS-SUPPRIMEES.md   code de migration retiré
```

Fichiers de l'amont supprimés (inutiles ici ou référençant les dépôts et images de Pumpkin) : CI GitHub, `Dockerfile`, `docker-compose.yml`, `egg-pumpkin.json`, fichiers Nix, `CODE_OF_CONDUCT.md`, `CONTRIBUTING.md`, `SECURITY.md`, `.devcontainer`, grande animation du README d'origine. Les mentions de droits d'auteur sont intactes.

## Releases

Tag `v<version>`, publiée en **pré-release** avec le binaire statique, `SHA256SUMS.txt`, `RELEASE-NOTES.md`, `LICENSE`, `NOTICE.md` et `CHANGES.md`. Procédure : `docs/RELEASES.md`.
