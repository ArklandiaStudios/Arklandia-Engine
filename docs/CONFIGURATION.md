# Configuration d'Arklandia Engine

Le fichier de configuration s'appelle toujours **`pumpkin.toml`** (même nom et même format que Pumpkin, pour garder les outils et les plugins existants). Il est créé au premier lancement dans le dossier courant. Un exemple complet et commenté est fourni : `docs/pumpkin.toml.example`. Seules les sections propres à Arklandia sont détaillées ici ; pour les autres, voir les commentaires du fichier généré.

## Mise à jour automatique

À chaque démarrage, le moteur compare `pumpkin.toml` aux valeurs par défaut de la version installée :

- les clés manquantes sont ajoutées **avec leur valeur par défaut** (à la fin de leur section, ou la section entière à la fin du fichier) ; vos valeurs, vos commentaires et l'ordre du fichier ne changent jamais ;
- le fichier n'est réécrit que si quelque chose a été ajouté, de manière atomique, après une sauvegarde `pumpkin.toml.bak-AAAAMMJJ-HHMMSS` (les 3 plus récentes sont gardées) ; une ligne de console l'indique ;
- une clé inconnue est conservée et ignorée (signalée au niveau `debug`) ;
- `config_version` est écrit automatiquement ; un fichier de version plus récente que le moteur n'est jamais modifié (avertissement) ;
- fichier en lecture seule ou ajout impossible à vérifier : avertissement et valeurs par défaut utilisées en mémoire seulement ;
- une valeur invalide arrête le démarrage avec un message explicite, sans réécrire le fichier.

## `[operators]`

```toml
[operators]
enabled = true     # false : /op et /deop désactivés, personne n'est opérateur
```

Avec `enabled = false`, aucun joueur ne reçoit de droits d'opérateur, même s'il figure dans la liste des opérateurs. Les droits passent alors par un plugin de permissions.

## `[analytics]`

```toml
[analytics]
enabled = false
```

Remplace la section `[telemetry]` de Pumpkin. Réservée à une fonction future : **aucune donnée n'est collectée ni envoyée**, quelle que soit la valeur.

## `[server_links]`

```toml
[server_links]
enabled = true                          # liens envoyés au menu du client
bug_report = "https://exemple.net/bug"  # /bug
discord = "https://discord.gg/xxxx"     # /discord
website = "https://exemple.net"         # /site
announcements = "&7Bienvenue ! &f/discord"  # chat, à chaque joueur qui rejoint ; vide = rien
[server_links.custom]
Wiki = "https://exemple.net/wiki"
```

Les commandes `/bug`, `/discord` et `/site` existent seulement si leur lien est non vide (indépendamment de `enabled`, qui ne règle que le menu du client). Les plugins lisent ces liens par l'interface `server-links` (`server-links()`, `get-server-link("discord")`).

## `[messages]`

Messages d'arrivée et de départ. **Un champ vide = rien n'est envoyé** (et le message natif « X joined the game » est supprimé).

```toml
[messages]
join = "&8[&a+&8] &7{player}"
quit = "&8[&c-&8] &7{player}"
first_join = "&8[&6★&8] &7Bienvenue à &f{player}&7 sur Arklandia !"   # remplace join à la 1re connexion
welcome = ""                                                           # privé, 1re connexion seulement ; \n = saut de ligne
```

- Format : `&0-&9 &a-&f` (couleurs), `&k &l &m &n &o` (styles), `&r` (réinitialiser), `&#RRGGBB` (hexadécimal), `&&` (un `&` littéral), `\n`.
- Variables : `{player}`, `{displayname}`, `{online}` (le joueur qui arrive compte, celui qui part non), `{max}`, `{world}` (`overworld`, `the_nether`…), `{count}` (nombre de joueurs déjà passés).
- « Première connexion » : aucun fichier `<monde>/players/data/<uuid>.dat` à la connexion. Avec `player_data.save_player_data = false`, tous les joueurs sont « nouveaux » à chaque connexion.
- Les annonces vont à tous les joueurs de tous les mondes et sont écrites dans la console. `PlayerJoinEvent` / `PlayerLeaveEvent` restent émis : un plugin peut les annuler (vanish) ou changer le texte, sauf si le champ est vide.

## `[private_messages]`

```toml
[private_messages]
enabled = true                  # false : retour au /msg vanilla (pas de /r, pas de hook)
format_sent = "&8[&7Vous &8→ &f{target}&8] &7{message}"
format_received = "&8[&f{sender} &8→ &7Vous&8] &7{message}"
sound = ""                      # son joué au destinataire, ex. "minecraft:block.note_block.pling"
reply_hint = "&7Utilisation : &f/r <message>"
not_found = "&cJoueur introuvable."
no_reply_target = "&cPersonne à qui répondre."
reply_offline = "&c{target} s'est déconnecté."
self_message = "&cVous ne pouvez pas vous écrire à vous-même."
empty_message = "&cLe message est vide."
spy_plugin = ""                 # plugin consulté avant chaque message (hook IPC), ex. "ArkStaff"
spy_timeout_ms = 250
```

Commandes : `/msg` `/tell` `/w` `/whisper` `/m <joueur> <message>` et `/r` `/reply <message>`. Un texte d'erreur vide prend la valeur interne par défaut. Permission `pumpkin.msg.color` (refusée par défaut) : autorise les codes `&` dans les messages du joueur. `minecraft:command.reply` (niveau 0) règle `/r`. Les messages privés sont masqués dans la console du serveur.

### Hook IPC `spy_plugin`

Si `spy_plugin` désigne un plugin chargé, le moteur appelle sa fonction `handle-ipc-message` **avant chaque envoi**, avec l'expéditeur IPC `"Pumpkin"`, après ses propres validations (destinataire en ligne, pas soi-même, message non vide). L'appel est synchrone et limité à `spy_timeout_ms`.

Requête (JSON UTF-8) :

```json
{"op":"pm","from":"<uuid>","from_name":"Alice","to":"<uuid>","to_name":"Bob","message":"texte brut"}
```

Réponse attendue : `{"allow":true}` ou `{"allow":false,"reason":"…"}`. Avec `allow:false`, le message n'est pas envoyé et l'expéditeur voit `reason` (format `&`, variable `{target}`), ou `not_found` si `reason` est vide ou vaut `"not_found"`. Plugin absent, erreur, délai dépassé ou réponse illisible : **le message passe normalement**. ArkStaff s'en sert pour `/socialspy`, le vanish (« introuvable ») et le mute.

## `[plugins]`

```toml
[plugins]
enabled = true
allowed_permissions = ["…"]    # approuvées pour tous les plugins, sans question en console
blocked_permissions = []
world_edit_threads = 0         # threads du pool d'édition de blocs ; 0 = max(1, cœurs - 2)
```

- Les permissions demandées par un plugin sont accordées d'après `allowed_permissions` / `blocked_permissions` (globales) et `[plugins.overrides.<Nom>]` (par plugin) ; le moteur ne pose jamais de question.
- `world_edit_threads` : taille du pool dédié `ArkEdit-N` qui traite les lectures et écritures de blocs en masse demandées par les plugins (ArkWorld). Ces threads sont distincts du thread de tick, du runtime asynchrone et du pool global.

## `[commands.overrides.<nom>]`

```toml
[commands.overrides.gamemode]
permission_level = 4      # 0 tous, 2 opérateurs, 3 admins, 4 propriétaire

[commands.overrides.tp]
enabled = false           # commande masquée : « commande inconnue »
```

Les commandes de plugin qui portent le même nom qu'une commande native **la remplacent** (sans fusion d'arguments) et lèvent aussi son `enabled = false`. Les alias ne sont pas remplacés : un plugin qui remplace `tp` et `teleport` doit enregistrer les deux.

## Identité du serveur

Le moteur annonce la marque `Arklandia Engine` au client (`minecraft:brand`, visible dans F3). Le MOTD par défaut est `Un serveur propulsé par Arklandia Engine` ; il se change dans `pumpkin.toml` (`motd`).
