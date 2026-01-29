from collections.abc import Mapping
from typing import Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="ModelInfo")


@_attrs_define
class ModelInfo:
    """
    Attributes:
        id (str):
        name (str):
        modality (str):
        tier (str):
        worker_type (str):
        provider (str):
        health_status (str):
        last_error (Union[Unset, str]):
    """

    id: str
    name: str
    modality: str
    tier: str
    worker_type: str
    provider: str
    health_status: str
    last_error: Union[Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        id = self.id

        name = self.name

        modality = self.modality

        tier = self.tier

        worker_type = self.worker_type

        provider = self.provider

        health_status = self.health_status

        last_error = self.last_error

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "id": id,
                "name": name,
                "modality": modality,
                "tier": tier,
                "worker_type": worker_type,
                "provider": provider,
                "health_status": health_status,
            }
        )
        if last_error is not UNSET:
            field_dict["last_error"] = last_error

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        id = d.pop("id")

        name = d.pop("name")

        modality = d.pop("modality")

        tier = d.pop("tier")

        worker_type = d.pop("worker_type")

        provider = d.pop("provider")

        health_status = d.pop("health_status")

        last_error = d.pop("last_error", UNSET)

        model_info = cls(
            id=id,
            name=name,
            modality=modality,
            tier=tier,
            worker_type=worker_type,
            provider=provider,
            health_status=health_status,
            last_error=last_error,
        )

        model_info.additional_properties = d
        return model_info

    @property
    def additional_keys(self) -> list[str]:
        return list(self.additional_properties.keys())

    def __getitem__(self, key: str) -> Any:
        return self.additional_properties[key]

    def __setitem__(self, key: str, value: Any) -> None:
        self.additional_properties[key] = value

    def __delitem__(self, key: str) -> None:
        del self.additional_properties[key]

    def __contains__(self, key: str) -> bool:
        return key in self.additional_properties
