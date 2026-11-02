# Migrations supprimées

Aucune version d'Arklandia Engine n'a été publiée avant la 0.1.0 : le code dont le seul but était de migrer depuis une version antérieure de notre moteur a été retiré. Les fonctions héritées de Pumpkin ne sont pas concernées.

## Retiré

| Élément | Détail |
| --- | --- |
| `pumpkin-config/src/migrate.rs` | mécanisme de « migrations nommées » : structure `Migration`, liste `MIGRATIONS` (vide), fonction `run_migrations` et son branchement dans le chargement de `pumpkin.toml` (réécriture complète du fichier après migration) |
| `[server_links]` | ancien nom `support` de la clé `discord` (lecture de l'ancien champ et fonction `discord_link`) |
| `arkbug-build.json` | manifeste de construction des versions internes `arklandia1` / `arklandia2` ; aucun code ne le lit (la liste des correctifs est dans `CHANGES.md` et dans la commande `/arkbug`) |
| Fichiers de livraison internes | `CHANGES-bans.md` des étapes ArkStaff, scripts de test propres à l environnement de développement : leur contenu utile est fusionné dans `CHANGES.md` |

## Conservé, et pourquoi

| Élément | Raison |
| --- | --- |
| Fusion automatique des valeurs manquantes de `pumpkin.toml` + sauvegarde `pumpkin.toml.bak-…` | fonctionnalité : une version qui ajoute des réglages complète le fichier de l'utilisateur sans toucher à ses valeurs |
| Comparaison de `config_version` (fichier plus récent laissé intact, avertissement) | protection |
| Lecture des formats de données de Pumpkin et de Minecraft (anciens mondes Anvil, etc.) | fonctions d'amont |
