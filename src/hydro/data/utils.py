from typing import Callable, NamedTuple

import numpy as np
import numpy.typing as npt
import polars as pl

from .hydro import Metadata


class Model(NamedTuple):
    init: Callable[[], tuple[npt.NDArray[np.float64], npt.NDArray[np.float64]]]
    simulate: Callable[
        [npt.NDArray[np.float64], pl.DataFrame, Metadata],
        npt.NDArray[np.float64],
    ]


class Data(NamedTuple):
    precipitation: npt.NDArray[np.float64]
    temperature: npt.NDArray[np.float64]
    pet: npt.NDArray[np.float64]
    day_of_year: npt.NDArray[np.uintp]
