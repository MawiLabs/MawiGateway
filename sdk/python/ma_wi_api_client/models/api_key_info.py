from collections.abc import Mapping
from typing import Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="ApiKeyInfo")


@_attrs_define
class ApiKeyInfo:
    """
    Attributes:
        id (str):
        name (str):
        prefix (str):
        created_at (str):
        expires_at (Union[Unset, str]):
        last_used_at (Union[Unset, str]):
    """

    id: str
    name: str
    prefix: str
    created_at: str
    expires_at: Union[Unset, str] = UNSET
    last_used_at: Union[Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        id = self.id

        name = self.name

        prefix = self.prefix

        created_at = self.created_at

        expires_at = self.expires_at

        last_used_at = self.last_used_at

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "id": id,
                "name": name,
                "prefix": prefix,
                "created_at": created_at,
            }
        )
        if expires_at is not UNSET:
            field_dict["expires_at"] = expires_at
        if last_used_at is not UNSET:
            field_dict["last_used_at"] = last_used_at

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        id = d.pop("id")

        name = d.pop("name")

        prefix = d.pop("prefix")

        created_at = d.pop("created_at")

        expires_at = d.pop("expires_at", UNSET)

        last_used_at = d.pop("last_used_at", UNSET)

        api_key_info = cls(
            id=id,
            name=name,
            prefix=prefix,
            created_at=created_at,
            expires_at=expires_at,
            last_used_at=last_used_at,
        )

        api_key_info.additional_properties = d
        return api_key_info

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
