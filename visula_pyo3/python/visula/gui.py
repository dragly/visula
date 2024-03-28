from dataclasses import dataclass


@dataclass
class Slider:
    minimum: float
    maximum: float
    step: float
