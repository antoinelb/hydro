from typing import Protocol

import numpy as np
import numpy.typing as npt

class Data(Protocol):
    precipitation: npt.NDArray[np.float64]
    temperature: npt.NDArray[np.float64]
    pet: npt.NDArray[np.float64]
    day_of_year: npt.NDArray[np.uintp]

class Metadata(Protocol):
    elevation_layers: npt.NDArray[np.float64]
    median_elevation: float
