import numpy as np
from numpy.typing import NDArray

def simulate(
    temperature: NDArray[np.float64],
    day_of_year: NDArray[np.float64],
    latitude: float,
) -> NDArray[np.float64]: ...
