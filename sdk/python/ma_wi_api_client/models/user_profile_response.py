from collections.abc import Mapping
from typing import Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="UserProfileResponse")


@_attrs_define
class UserProfileResponse:
    """
    Attributes:
        id (str):
        email (str):
        tier (str):
        monthly_quota_usd (float):
        current_usage_usd (float):
        is_free_tier (bool):
        name (Union[Unset, str]):
    """

    id: str
    email: str
    tier: str
    monthly_quota_usd: float
    current_usage_usd: float
    is_free_tier: bool
    name: Union[Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        id = self.id

        email = self.email

        tier = self.tier

        monthly_quota_usd = self.monthly_quota_usd

        current_usage_usd = self.current_usage_usd

        is_free_tier = self.is_free_tier

        name = self.name

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "id": id,
                "email": email,
                "tier": tier,
                "monthly_quota_usd": monthly_quota_usd,
                "current_usage_usd": current_usage_usd,
                "is_free_tier": is_free_tier,
            }
        )
        if name is not UNSET:
            field_dict["name"] = name

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        id = d.pop("id")

        email = d.pop("email")

        tier = d.pop("tier")

        monthly_quota_usd = d.pop("monthly_quota_usd")

        current_usage_usd = d.pop("current_usage_usd")

        is_free_tier = d.pop("is_free_tier")

        name = d.pop("name", UNSET)

        user_profile_response = cls(
            id=id,
            email=email,
            tier=tier,
            monthly_quota_usd=monthly_quota_usd,
            current_usage_usd=current_usage_usd,
            is_free_tier=is_free_tier,
            name=name,
        )

        user_profile_response.additional_properties = d
        return user_profile_response

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
