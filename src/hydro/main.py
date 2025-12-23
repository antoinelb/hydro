from pathlib import Path

import uvicorn
from starlette.applications import Starlette

from . import api, config
from .logging import init_logging, logger

##########
# public #
##########


def create_app() -> Starlette:
    init_logging()

    app = Starlette(debug=config.DEBUG, routes=api.get_routes())

    logger.info("App started.")
    if config.DEBUG:
        logger.warning("Running in debug mode.")

    return app


def run_server() -> None:
    """
    Run the dashboard server.

    This function starts the Uvicorn server with the configured settings
    for host, port, and reload behavior. It also initializes logging and
    sets the appropriate log level based on debug mode.
    """
    init_logging()

    logger.info(
        f"Starting app in {'debug' if config.DEBUG else 'production'} mode "
        f"on port {config.PORT}"
    )

    uvicorn.run(
        "hydro.main:create_app",
        factory=True,
        host=config.HOST,
        port=config.PORT,
        reload=config.RELOAD,
        reload_dirs=str((Path(__file__).parent / "..").absolute()),
        log_level="debug" if config.DEBUG else "info",
        access_log=True,
    )
