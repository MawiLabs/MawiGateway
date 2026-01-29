from collections.abc import Mapping
from typing import Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="UpdateProvider")


@_attrs_define
class UpdateProvider:
    """
    Attributes:
        name (Union[Unset, str]):
        provider_type (Union[Unset, str]):
        api_endpoint (Union[Unset, str]):
        api_version (Union[Unset, str]):
        api_key (Union[Unset, str]):
        description (Union[Unset, str]):
        icon_url (Union[Unset, str]):
    """

    name: Union[Unset, str] = UNSET
    provider_type: Union[Unset, str] = UNSET
    api_endpoint: Union[Unset, str] = UNSET
    api_version: Union[Unset, str] = UNSET
    api_key: Union[Unset, str] = UNSET
    description: Union[Unset, str] = UNSET
    icon_url: Union[Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        name = self.name

        provider_type = self.provider_type

        api_endpoint = self.api_endpoint

        api_version = self.api_version

        api_key = self.api_key

        description = self.description

        icon_url = self.icon_url

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if name is not UNSET:
            field_dict["name"] = name
        if provider_type is not UNSET:
            field_dict["provider_type"] = provider_type
        if api_endpoint is not UNSET:
            field_dict["api_endpoint"] = api_endpoint
        if api_version is not UNSET:
            field_dict["api_version"] = api_version
        if api_key is not UNSET:
            field_dict["api_key"] = api_key
        if description is not UNSET:
            field_dict["description"] = description
        if icon_url is not UNSET:
            field_dict["icon_url"] = icon_url

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        name = d.pop("name", UNSET)

        provider_type = d.pop("provider_type", UNSET)

        api_endpoint = d.pop("api_endpoint", UNSET)

        api_version = d.pop("api_version", UNSET)

        api_key = d.pop("api_key", UNSET)

        description = d.pop("description", UNSET)

        icon_url = d.pop("icon_url", UNSET)

        update_provider = cls(
            name=name,
            provider_type=provider_type,
            api_endpoint=api_endpoint,
            api_version=api_version,
            api_key=api_key,
            description=description,
            icon_url=icon_url,
        )

        update_provider.additional_properties = d
        return update_provider

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
