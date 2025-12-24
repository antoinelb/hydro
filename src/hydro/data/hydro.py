import polars as pl

from hydro.utils import paths

##########
# public #
##########


def read_stations() -> pl.DataFrame:
    url = "https://www.donneesquebec.ca/recherche/dataset/c31e2bee-a899-46ca-ad84-5798f0f49676/resource/6b2d32ef-80e2-445b-9bd1-97ddc39b5d59/download/stations_hydrometriques.csv"
    path = paths.data_dir / "raw" / "hydro" / "stations.ipc"
    if path.exists():
        data = pl.read_ipc(path)
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        data = (
            pl.read_csv(url, schema_overrides={"no": pl.String})
            .select(
                pl.col("no").alias("id"),
                pl.col("nom").alias("name"),
                "type",
                pl.col("latitude").alias("lat"),
                pl.col("longitude").alias("lon"),
                pl.col("debut").cast(pl.Int64, strict=False).alias("start"),
                pl.col("fin").cast(pl.Int64, strict=False).alias("end"),
                pl.col("cours_eau").alias("waterway"),
                pl.col("superficie").alias("area"),
                (pl.col("regime") == "Influencé").alias("influenced"),
                (pl.col("lien_historique") == "www.eau.ec.gc.ca").alias(
                    "federal"
                ),
            )
            .filter(
                pl.col("type").str.contains("Débit")
                & ~pl.col("federal")
                & pl.col("end").is_null()
            )
            .drop("type", "federal")
            .with_columns(
                (pl.col("name") + " (" + pl.col("id") + ")").alias("station")
            )
        )
        data.write_ipc(path)
    return data
