# Publication des versions

Le binaire n'est **jamais** commité : il est joint aux Releases GitHub du dépôt privé.

## Nommage

- tag : `v<version>`, par exemple `v0.1.0` ;
- titre : `Arklandia Engine v<version> (pré-version)` ;
- chaque Release est marquée **pré-release** (`--prerelease`) tant que le moteur n'est pas stable ;
- fichier : `arklandia-engine-<version>-linux-x86_64-musl` (binaire statique, ≈ 130 Mo).

## Préparer les fichiers

```sh
scripts/make-release.sh              # compile puis prépare release/arklandia-engine-<version>/
SKIP_BUILD=1 scripts/make-release.sh # réutilise le binaire déjà compilé
```

Le dossier contient : le binaire, `SHA256SUMS.txt`, `RELEASE-NOTES.md`, `LICENSE`, `NOTICE.md`, `CHANGES.md`, `pumpkin.toml.example`. `release/` est ignoré par Git. La version est lue dans `[workspace.package]` de `Cargo.toml` (`0.1.0+mc26.3` : la partie après `+` est omise du nom de fichier).

## Publier

```sh
git tag v0.1.0
git push origin v0.1.0
D=release/arklandia-engine-0.1.0
gh release create v0.1.0 --prerelease --title "Arklandia Engine v0.1.0 (pré-version)" \
  --notes-file "$D/RELEASE-NOTES.md" \
  "$D"/arklandia-engine-0.1.0-linux-x86_64-musl "$D"/SHA256SUMS.txt "$D"/RELEASE-NOTES.md \
  "$D"/LICENSE "$D"/NOTICE.md "$D"/CHANGES.md "$D"/pumpkin.toml.example
```

## Rappel de licence

Le moteur est sous GPL v3. Distribuer le binaire à une personne extérieure à Arklandia (y compris par une Release rendue publique) oblige à lui fournir le code source complet correspondant, sous la même licence.

## Avant de publier

Mettre à jour la version (`[workspace.package]` de `Cargo.toml`, `Cargo.lock`), ajouter les changements à `CHANGES.md`, lancer `scripts/build.sh test`, vérifier que les plugins Ark se chargent (`Loaded N plugins`).
