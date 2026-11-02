# Arklandia Engine : modifications par rapport à Pumpkin

Base : Pumpkin (https://github.com/Pumpkin-MC/Pumpkin), commit `204a94ed895f041a845630d5a23c094705a0704e` (état du 2026-09-20), Minecraft Java 26.3 (protocole 777), Bedrock 1.26.51.

Ce fichier regroupe, par thème, toutes les modifications faites par Arklandia ; les dates sont celles des livraisons internes (intégration initiale le 2026-10-28, moteur final le 2026-10-29, étapes ArkStaff du 2026-10-30 au 2026-11-01). Tous les changements sont publiés sous GNU GPL v3 comme le reste du code (voir `NOTICE.md`). Aucune signature WIT existante ni le variant `event` n'est modifié : les plugins Ark 0.1.0 se chargent sans recompilation.

## 0.1.0+mc26.3 : renommage (2026-11-02)

- Produit renommé **Arklandia Engine**, version repartie de `0.1.0` (build `0.1.0+mc26.3`, anciennement `0.2.0+26.3-26.51.arklandia2`).
- Binaire `arklandia-engine` (`[[bin]]` du crate `pumpkin`), bannière `Starting Arklandia Engine <version> (basé sur Pumpkin) Java Minecraft (Protocol 777)`, marque réseau `minecraft:brand` « Arklandia Engine » (écran F3), messages d'avertissement de build debug, rapport de crash, `/arkbug`, MOTD par défaut, nom de serveur Bedrock.
- Conservés pour la compatibilité : crates `pumpkin-*`, paquet WIT `pumpkin:plugin`, API `pumpkin_plugin_api`, espaces de noms et permissions `pumpkin:…`, `pumpkin.toml`, dossiers de données, expéditeur IPC `"Pumpkin"`.
- Le cache des plugins compilés (`.cwasm`) porte la version dans son nom : il est reconstruit une fois après le changement de version.
- Retrait du code de migration propre aux anciennes versions du moteur (`MIGRATIONS-SUPPRIMEES.md`).

## Configuration

### Lecture de `pumpkin.toml` sans plantage et complétion automatique (2026-10-28, révisé 2026-10-31)

- Une clé inconnue ou manquante ne fait plus échouer le démarrage : fusion profonde entre les valeurs par défaut et celles de l'utilisateur (valeurs et tableaux de l'utilisateur conservés, clés inconnues conservées et signalées en `debug`).
- Le fichier n'est réécrit que si une clé a été ajoutée, **par insertion de texte** : commentaires, ordre et mise en forme sont conservés ; une clé manquante est ajoutée à la fin de sa section, une section manquante à la fin du fichier. Le résultat est relu et doit être égal à « défauts + valeurs de l'utilisateur », sinon le fichier n'est pas touché (avertissement, fusion en mémoire).
- Écriture atomique après sauvegarde `pumpkin.toml.bak-AAAAMMJJ-HHMMSS` (les 3 dernières sont gardées). Nouvelle clé racine `config_version = 1` ; un fichier de version plus récente n'est jamais modifié. Fichiers : `crates/pumpkin-config/src/migrate.rs`, `lib.rs`.

### Bedrock désactivable (2026-10-28)

- `[networking.bedrock] enabled = false` désactive le protocole Bedrock (option déjà présente dans Pumpkin) ; la bannière de démarrage n'annonce la version Bedrock que lorsqu'il est activé.

### Télémétrie remplacée par `[analytics]` (2026-10-28)

- La section `[telemetry]` de Pumpkin et l'envoi de statistiques (`start_telemetry`) sont supprimés, ainsi que l'avertissement de liens de support affiché au démarrage. `[analytics] enabled` est réservée à une fonction future : **rien n'est collecté ni envoyé**, quelle que soit sa valeur.

### Section `[operators]` (2026-10-28)

- `[operators] enabled = false` : `/op` et `/deop` sont désactivés et personne ne reçoit de droits d'opérateur, quel que soit le contenu de la liste des opérateurs (`crates/pumpkin-config/src/operators.rs`).

## Plugins

### Permissions de plugins accordées automatiquement (2026-10-28)

- Suppression de la question en console (« autoriser les permissions demandées ? ») et de la vérification de signature de marketplace des plugins : les permissions sont accordées d'après `allowed_permissions` / `blocked_permissions` de `[plugins]` et de `[plugins.overrides.<Nom>]`, sans interaction (`plugin/mod.rs`, `wasm_host/mod.rs`).

### Journaux de chargement groupés (2026-10-28)

- Le chargement initial écrit une seule ligne `Loaded N plugins: Nom (version), …` sans avertissement de permissions.

### Compilation parallèle wasmtime (2026-10-28)

- Option wasmtime `parallel-compilation` : les plugins sont compilés en parallèle au démarrage.

### Événements pour ArkGuard : interface `event-extra` (2026-10-28)

- Interface WIT facultative `event-extra` : le contexte supplémentaire de l'événement en cours (monde, cause, type d'entité, listes de blocs…) se lit par des fonctions dédiées, car les enregistrements d'événements sont figés pour garder les plugins compilés chargeables.

### Interface `server-links` (2026-10-30)

- Interface WIT facultative en lecture seule sur `[server_links]` : `server-links()` et `get-server-link(nom)` (`bug`, `discord`, `site` et liens personnalisés, valeurs vides omises). Les plugins compilés avant elle ne l'importent pas et se chargent toujours.

### Interfaces `blocks` et `database` (2026-10-28)

- `blocks` : conversion d'un état de bloc en texte (`minecraft:oak_stairs[facing=north]`) et inversement.
- `database` : accès aux bases de données externes effectué par le serveur au nom du plugin (permissions `network.tcp.connect` et `network.dns` requises ; connexions fermées au déchargement du plugin).

### Remplacement d'une commande native par un plugin (2026-10-30)

- Une commande de plugin qui porte le nom d'un nœud racine d'une autre source (commande native ou autre plugin) **remplace** ce nœud au lieu de fusionner ses arguments avec lui (`pumpkin-command/src/node/dispatcher.rs`, `tree.rs`). Elle lève aussi une désactivation par `[commands.overrides.<nom>] enabled = false`. Deux plugins avec le même nom : le dernier chargé l'emporte. Les alias ne sont pas remplacés (enregistrer `tp` et `teleport` séparément).
- `[commands.overrides.<nom>]` : `enabled` et `permission_level` par commande.

## Annonces et liens

### Liens du serveur (2026-10-28)

- `[server_links]` : `bug_report`, `discord`, `website`, `announcements` et liens `custom`. Les commandes `/bug`, `/discord` et `/site` n'existent que si leur lien est renseigné ; les liens sont aussi envoyés au menu du client. Les URL par défaut de Pumpkin (dépôt d'issues) sont remplacées par des valeurs vides. L'ancien nom `support` de `discord` n'est plus lu.
- `announcements` : message envoyé dans le chat à chaque joueur qui rejoint (rien tant qu'il est vide).

### Messages d'arrivée, de départ et de bienvenue (2026-10-31)

- Section `[messages]` : `join`, `quit`, `first_join`, `welcome`. Format `&` (couleurs, styles, `&#RRGGBB`, `&&`, `\n`), variables `{player}`, `{displayname}`, `{online}`, `{max}`, `{world}`, `{count}`. Un champ vide n'envoie rien et supprime le message natif « X joined the game ».
- `PlayerJoinEvent` / `PlayerLeaveEvent` sont toujours émis (annulables, `join_message` / `leave_message` modifiables) ; annonces envoyées à tous les mondes et journalisées dans la console. Première connexion = pas de fichier `players/data/<uuid>.dat`.
- Module de format partagé `pumpkin-util/src/text/markup.rs` (`parse`, `render`, `expand`, `escape`, `strip`) avec tests ; les valeurs de variables sont échappées (un pseudo ne peut pas injecter de format).

### Messages privés et hook IPC (2026-11-01)

- `/msg` `/tell` `/w` `/whisper` `/m` et `/r` `/reply` remplacent les commandes natives : format, son, textes d'erreur et activation dans `[private_messages]`. Dernier correspondant tenu en mémoire (à l'envoi et à la réception). Permissions `minecraft:command.reply` (niveau 0) et `pumpkin.msg.color` (refusée par défaut : sans elle, les codes `&` du joueur sont affichés tels quels).
- Les messages privés ne sont pas écrits dans la console (« msg [private message hidden] »).
- Hook IPC : avant l'envoi, le moteur interroge le plugin `spy_plugin` (appel synchrone, délai `spy_timeout_ms`, 250 ms par défaut) avec l'expéditeur IPC `"Pumpkin"` et la requête `{"op":"pm",…}` ; réponse `{"allow":true}` ou `{"allow":false,"reason":"…"}`. Plugin absent, erreur ou délai dépassé : le message passe. Aucun changement WIT.

## Performance et édition de blocs

### Pool d'édition de blocs `ArkEdit` et mimalloc (2026-10-29)

- `[plugins] world_edit_threads` (0 = automatique `max(1, cœurs - 2)`) : pool rayon dédié `ArkEdit-N` pour `get_block_state_ids` / `set_block_state_ids` (lecture et écriture par chunk en parallèle, effets des blocs en parallèle, lumière en une passe par lot, un `CMultiBlockUpdate` par section) (`world/bulk_edit.rs`, `lighting/runtime.rs`, `wasm_host/wit/v0_1/world.rs`).
- Les chunks restent tenus 45 s après le dernier usage (évite le déchargement puis la régénération entre deux lots).
- Allocateur mimalloc sur la cible musl (le `malloc` de musl a un verrou global).
- Différence observable : les mises à jour de voisins d'un lot voient l'état final du lot entier (comme `/fill`).

## Commandes et comportements

### Correctif du chat pour les commandes en `//` (2026-10-28)

- `net/java/play/chat_command.rs` : `//pos1`, `//wand`, etc. sont transmis comme commandes.

### Lapins et créatif (2026-10-28)

- Les mobs ne fuient plus et ne ciblent plus un joueur en créatif (`avoid_entity.rs`, `mob/mod.rs`, `melee_attack.rs`) ; correctifs des lapins.

### Message d'attributs (2026-10-30)

- `entity/living.rs` : l'avertissement géant « Entity type … has no base value for attribute N » (plus de 3 000 caractères, à chaque occurrence) devient un message court de niveau `debug`.

### Commande `/arkbug`

- Affiche la version du moteur et la liste des correctifs de comportement intégrés (références `#3844`, `#3759`, `#4048`, `#3516`, `#4089`, `#3972`, `#4123`, `#3847`, `#3918`, `#3933`, `#3969`, `#4063`, `#4090`, `#3989`, `#3065`, `#3716`, `#3698`, `#3730`, `#2016`, `#4045`, `#3902`, `#3571`, `#3510`, `#3367`, `#3511`, `#3512`, `#3567`, `#4024`, `#4075`, `#4046`). Ces correctifs touchent notamment : blocs (lit, feu, portail du Nether, coffres, shulker, TNT, tripwire, vault…), blocs-entités (hopper, four, spawners…), mobs et poissons (`fish_land.rs`, `fish_swim.rs`, `squid_swim.rs`), cultures, `/execute` et `/function` (budget d'exécution : `datapack/function_chain.rs`), mises à jour de voisins (`world/neighbor_updater.rs`), inventaires visibles sans copie (`entity/inspection.rs`).

### Autres modifications d'ArkStaff côté moteur (2026-10-30 / 2026-11-01)

- Corrections destinées à ArkStaff : pas de diffusion aux observateurs d'une téléportation annulée, permission d'acteur de téléportation limitée à l'événement, visibilité des entités et de la liste des joueurs par observateur, nombre de joueurs et échantillon publics filtrés, sauvegardes d'inventaire sans perte et sauvegardes de joueurs durables, synchronisation correcte des emplacements d'écran.
- Autosave : `autosave_ticks` (6000 par défaut) peut bloquer le tick plus de 30 s quand beaucoup de chunks ont été générés ; à surveiller, ou à désactiver en test (`autosave_ticks = 0`).
