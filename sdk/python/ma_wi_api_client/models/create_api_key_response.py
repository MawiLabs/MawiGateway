from collections.abc import Mapping
from typing import Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="CreateApiKeyResponse")


@_attrs_define
class CreateApiKeyResponse:
    """
    Attributes:
        id (str):
        prefix (str):
        name (str):
        raw_key (str):
        created_at (str):
        expires_at (Union[Unset, str]):
    """

    id: str
    prefix: str
    name: str
    raw_key: str
    created_at: str
    expires_at: Union[Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        id = self.id

        prefix = self.prefix

        name = self.name

        raw_key = self.raw_key

        created_at = self.created_at

        expires_at = self.expires_at

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "id": id,
                "prefix": prefix,
                "name": name,
                "raw_key": raw_key,
                "created_at": created_at,
            }
        )
        if expires_at is not UNSET:
            field_dict["expires_at"] = expires_at

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        id = d.pop("id")

        prefix = d.pop("prefix")

        name = d.pop("name")

        raw_key = d.pop("raw_key")

        created_at = d.pop("created_at")

        expires_at = d.pop("expires_at", UNSET)

        create_api_key_response = cls(
            id=id,
            prefix=prefix,
            name=name,
            raw_key=raw_key,
            created_at=created_at,
            expires_at=expires_at,
        )

        create_api_key_response.additional_properties = d
        return create_api_key_response

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
