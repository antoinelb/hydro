from typing import Literal, assert_never

from ..utils import Model
from . import day_median, gr4j

#########
# types #
#########

ClimateModel = Literal["day_median", "gr4j", "bucket"]

##########
# public #
##########


def get_model(model: ClimateModel) -> Model:
    match (model):
        case "day_median":
            return Model(day_median.init, day_median.simulate)
        case "gr4j":
            return Model(gr4j.init, gr4j.simulate)
        case "bucket":
            raise NotImplementedError()
        case _:
            assert_never(model)  # type: ignore
