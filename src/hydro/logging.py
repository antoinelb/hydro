import logging
import logging.config
import re
import sys
from typing import Literal, Optional

import click

from . import config

logger_name = "hydro"
logger = logging.getLogger(logger_name)


def init_logging() -> None:
    current_loggers = logging.Logger.manager.loggerDict.keys()  # type: ignore

    logging.config.dictConfig(
        {
            "disable_existing_loggers": True,
            "formatters": {
                "simple": {
                    "format": "%(levelname)s - %(message)s",
                    "class": "hydro.logging.ColourFormatter",
                },
                "complete": {
                    "datefmt": "%Y-%m-%d %H:%M:%S",
                    "format": "%(asctime)s - "
                    "%(name)s - "
                    "%(levelname)s - "
                    "%(message)s",
                },
            },
            "filters": {"route": {"()": RouteFilter}},  # type: ignore
            "handlers": {
                "console": {
                    "class": "logging.StreamHandler",
                    "formatter": "simple",
                    "stream": "ext://sys.stdout",
                    "level": "DEBUG" if config.DEBUG else "INFO",
                    "filters": ["route"],
                },
            },
            "loggers": {
                **{
                    logger: {
                        "handlers": ["console"],
                        "level": "WARNING",
                        "propagate": False,
                    }
                    for logger in current_loggers
                },
                **{
                    logger_name: {
                        "handlers": ["console"],
                        "level": "DEBUG" if config.DEBUG else "INFO",
                        "propagate": True,
                    },
                    "uvicorn.access": {
                        "handlers": ["console"],
                        "level": "INFO",
                        "propagate": False,
                    },
                },
            },  # type: ignore
            "version": 1,
        }
    )


class ColourFormatter(logging.Formatter):  # pragma: no cover
    """
    Custom logging formatter that adds color to log level names.

    This formatter applies color coding to log level names in the console
    output to improve readability and visual distinction between different
    log levels.
    """

    level_name_colours = {
        logging.DEBUG: lambda level: click.style(str(level), fg="cyan"),
        logging.INFO: lambda level: click.style(str(level), fg="green"),
        logging.WARNING: lambda level: click.style(str(level), fg="yellow"),
        logging.ERROR: lambda level: click.style(str(level), fg="red"),
        logging.CRITICAL: lambda level: click.style(
            str(level), fg="bright_red"
        ),
    }

    def __init__(
        self: "ColourFormatter",
        fmt: Optional[str] = None,
        datefmt: Optional[str] = None,
        style: Literal["%", "{", "$"] = "%",
        use_colours: Optional[bool] = None,
    ) -> None:
        """
        Initialize the ColourFormatter.

        Parameters
        ----------
        fmt : Optional[str], default=None
            Format string for log messages
        datefmt : Optional[str], default=None
            Format string for dates
        style : Literal["%", "{", "$"], default="%"
            Style of the format string
        use_colours : Optional[bool], default=None
            Whether to use colors. If None, colors are used if stdout is a TTY
        """
        if use_colours in (True, False):
            self.use_colours = use_colours  # pragma: no cover
        else:
            self.use_colours = sys.stdout.isatty()
        super().__init__(fmt=fmt, datefmt=datefmt, style=style)

    def colour_level_name(
        self: "ColourFormatter", level_name: str, level_no: int
    ) -> str:
        """
        Apply color to a log level name based on its severity.

        Parameters
        ----------
        level_name : str
            Name of the log level to color
        level_no : int
            Numeric value of the log level

        Returns
        -------
        str
            Colored log level name
        """
        fct = self.level_name_colours.get(
            level_no,
            lambda level_name: str(  # pylint: disable=unnecessary-lambda
                level_name
            ),
        )  # pragma: no cover
        return fct(level_name)  # pragma: no cover

    def formatMessage(
        self: "ColourFormatter", record: logging.LogRecord
    ) -> str:
        """
        Format a log record with colored level name if colors are enabled.

        Parameters
        ----------
        record : logging.LogRecord
            Log record to format

        Returns
        -------
        str
            Formatted log message
        """
        if self.use_colours:
            record.levelname = self.colour_level_name(
                record.levelname, record.levelno
            )  # pragma: no cover
        return super().formatMessage(record)


class RouteFilter(logging.Filter):
    """
    Filter for removing common route access logs.

    This filter prevents logging of successful (200) GET requests to common routes
    like the homepage, static files, and ping endpoint to reduce log noise.
    """

    def __init__(self: "RouteFilter", *args: str, **kwargs: str) -> None:
        """
        Initialize the RouteFilter.

        Parameters
        ----------
        *args : str
            Positional arguments passed to the parent class
        **kwargs : str
            Keyword arguments passed to the parent class
        """
        super().__init__(*args, **kwargs)

    def filter(self: "RouteFilter", record: logging.LogRecord) -> bool:
        """
        Filter log records based on common routes.

        This method filters out successful (200) GET requests to common routes
        to reduce log verbosity.

        Parameters
        ----------
        record : logging.LogRecord
            Log record to filter

        Returns
        -------
        bool
            True if the record should be logged, False if it should be filtered out
        """
        routes = [
            "/ping",
            "/",
            "/static/scripts/.+.js",
            "/static/styles/.+.css",
        ]
        msg = record.getMessage()
        return all(
            re.search(f'"GET {route}(?:\\?\\S+)? HTTP/1.1" 200', msg) is None
            for route in routes
        )
