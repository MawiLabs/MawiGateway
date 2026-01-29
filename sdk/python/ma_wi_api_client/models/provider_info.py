from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="ProviderInfo")


@_attrs_define
class ProviderInfo:
    """
    Attributes:
        id (str):
        name (str):
        provider_type (str):
        has_api_key (bool):
    """

    id: str
    name: str
    provider_type: str
    has_api_key: bool
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        id = self.id

        name = self.name

        provider_type = self.provider_type

        has_api_key = self.has_api_key

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "id": id,
                "name": name,
                "provider_type": provider_type,
                "has_api_key": has_api_key,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        id = d.pop("id")

        name = d.pop("name")

        provider_type = d.pop("provider_type")

        has_api_key = d.pop("has_api_key")

        provider_info = cls(
            id=id,
            name=name,
            provider_type=provider_type,
            has_api_key=has_api_key,
        )

        provider_info.additional_properties = d
        return provider_info

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
