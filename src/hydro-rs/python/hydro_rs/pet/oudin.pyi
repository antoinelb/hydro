import numpy as np
import numpy.typing as npt

def simulate(
    temperature: npt.NDArray[np.float64],
    day_of_year: npt.NDArray[np.float64],
    latitude: float,
) -> npt.NDArray[np.float64]: ...
