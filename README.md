# Hydro

Preuve de concept d'une plateforme d'hydrologie opérationnelle pour toute station hydrométrique du Québec. Le projet démontre la chaîne complète de modélisation hydrologique, de l'acquisition des données à la calibration des modèles.

## Fonctionnalités implémentées

### Sources de données

- **Données hydrométriques** : Niveaux d'eau et débits des stations du CEHQ (cehq.gouv.qc.ca)
- **Données météorologiques** : Température et précipitations via l'API d'Environnement Canada
- **Bassins versants** : Contours des bassins versants depuis Données Québec
- **Modèle numérique de terrain** : Élévation via le service de données géospatiales du Canada

### Modèles hydrologiques (Rust)

- **GR4J** : Modèle pluie-débit à 4 paramètres
- **CemaNeige** : Module neige avec bandes d'élévation (3 paramètres)
- **Oudin** : Calcul de l'évapotranspiration potentielle (PET)

### Calibration

- **SCE-UA** : Algorithme d'optimisation Shuffled Complex Evolution
- **Métriques** : RMSE, NSE (Nash-Sutcliffe), KGE (Kling-Gupta)
- **Modèle de référence** : Médiane journalière par jour de l'année

### Interface

- **Carte interactive** : Sélection des stations avec Leaflet
- **Graphiques** : Visualisation des données avec D3.js
- **Communication temps réel** : WebSocket bidirectionnel
- **Calibration en direct** : Suivi de la progression de la calibration

## Architecture

### Backend (Python 3.13 / Starlette)

- Serveur async avec uvicorn
- Cache des données en format Arrow IPC
- Traitement des données avec Polars

### Extension Rust (hydro-rs)

- Modèles hydrologiques performants via PyO3
- Algorithme de calibration SCE-UA

### Frontend (JavaScript vanilla)

- Architecture Elm-like (Model-Update-View)
- Communication WebSocket

## Installation

### Prérequis

- Python 3.13+
- Rust (pour compiler l'extension hydro-rs)
- [uv](https://docs.astral.sh/uv/) (gestionnaire de paquets Python)

### Étapes

```bash
# Cloner le dépôt
git clone <url-du-repo>
cd hydro

# Installer les dépendances (inclut la compilation de l'extension Rust)
uv sync

# Configurer l'environnement (optionnel)
# Créer un fichier .env avec les variables suivantes :
# HOST=127.0.0.1
# PORT=8002
# DEBUG=1
# RELOAD=1

# Lancer l'application
hydro
```

## Commandes

```bash
# Lancer l'application
hydro

# Qualité du code
uv run black src/
uv run isort src/
uv run ruff check src/
uv run ty

# Installer les dépendances
uv sync

# Compiler l'extension Rust
make build-rs
```

## Prochaines étapes

- Validation du modèle sur données indépendantes
- Système de prévision
- Projections climatiques
