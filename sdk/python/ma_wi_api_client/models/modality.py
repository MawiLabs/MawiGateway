from enum import Enum


class Modality(str, Enum):
    AUDIO = "Audio"
    IMAGE = "Image"
    TEXT = "Text"
    VIDEO = "Video"

    def __str__(self) -> str:
        return str(self.value)
