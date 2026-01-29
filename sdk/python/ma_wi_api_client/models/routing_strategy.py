from enum import Enum


class RoutingStrategy(str, Enum):
    HEALTH = "Health"
    LEASTCOST = "LeastCost"
    LEASTLATENCY = "LeastLatency"
    NONE = "None"
    WEIGHTEDRANDOM = "WeightedRandom"

    def __str__(self) -> str:
        return str(self.value)
