from collections.abc import Mapping
from typing import Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="ServiceModelInfo")


@_attrs_define
class ServiceModelInfo:
    """
    Attributes:
        model_id (str):
        model_name (str):
        position (int):
        provider_id (str):
        modality (str):
        weight (Union[Unset, int]):
        is_healthy (Union[Unset, bool]):
        health_status (Union[Unset, str]):
    """

    model_id: str
    model_name: str
    position: int
    provider_id: str
    modality: str
    weight: Union[Unset, int] = UNSET
    is_healthy: Union[Unset, bool] = UNSET
    health_status: Union[Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        model_id = self.model_id

        model_name = self.model_name

        position = self.position

        provider_id = self.provider_id

        modality = self.modality

        weight = self.weight

        is_healthy = self.is_healthy

        health_status = self.health_status

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "model_id": model_id,
                "model_name": model_name,
                "position": position,
                "provider_id": provider_id,
                "modality": modality,
            }
        )
        if weight is not UNSET:
            field_dict["weight"] = weight
        if is_healthy is not UNSET:
            field_dict["is_healthy"] = is_healthy
        if health_status is not UNSET:
            field_dict["health_status"] = health_status

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        model_id = d.pop("model_id")

        model_name = d.pop("model_name")

        position = d.pop("position")

        provider_id = d.pop("provider_id")

        modality = d.pop("modality")

        weight = d.pop("weight", UNSET)

        is_healthy = d.pop("is_healthy", UNSET)

        health_status = d.pop("health_status", UNSET)

        service_model_info = cls(
            model_id=model_id,
            model_name=model_name,
            position=position,
            provider_id=provider_id,
            modality=modality,
            weight=weight,
            is_healthy=is_healthy,
            health_status=health_status,
        )

        service_model_info.additional_properties = d
        return service_model_info

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
