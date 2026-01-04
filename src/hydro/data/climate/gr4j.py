import numpy as np
import numpy.typing as npt
import polars as pl
from hydro_rs.climate import gr4j

from ..hydro import Metadata
from ..utils import Data


def init() -> tuple[npt.NDArray[np.float64], npt.NDArray[np.float64]]:
    # parameters: x1, x2, x3, x4
    return gr4j.init()


def simulate(
    params: npt.NDArray[np.float64], data: pl.DataFrame, metadata: Metadata
) -> npt.NDArray[np.float64]:
    _data = Data(
        data["precipitation"].to_numpy(),
        data["temperature"].to_numpy(),
        data["pet"].to_numpy(),
        data["date"].dt.ordinal_day().to_numpy().astype(np.uintp),
    )
    return gr4j.simulate(params, _data, metadata)
