from typing import Literal, assert_never

from ..utils import Model
from . import cemaneige

#########
# types #
#########

SnowModel = Literal["cemaneige"]

##########
# public #
##########


def get_model(model: SnowModel) -> Model:
    match (model):
        case "cemaneige":
            return Model(cemaneige.init, cemaneige.simulate)
        case _:
            assert_never(model)  # type: ignore
