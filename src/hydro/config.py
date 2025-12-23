import warnings

from starlette.config import Config

with warnings.catch_warnings():
    warnings.filterwarnings(
        "ignore", category=UserWarning, module="starlette.config"
    )
    config = Config(".env")

DEBUG = config("DEBUG", cast=bool, default=False)
RELOAD = config("RELOAD", cast=bool, default=False)
HOST = config("HOST", default="127.0.0.1")
PORT = config("PORT", cast=int, default=8000)
