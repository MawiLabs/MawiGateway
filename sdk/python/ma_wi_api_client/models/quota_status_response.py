from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="QuotaStatusResponse")


@_attrs_define
class QuotaStatusResponse:
    """
    Attributes:
        personal_quota (float):
        personal_used (float):
        personal_remaining (float):
        personal_percentage (int):
        org_quota_available (float):
        org_percentage (int):
        total_available (float):
    """

    personal_quota: float
    personal_used: float
    personal_remaining: float
    personal_percentage: int
    org_quota_available: float
    org_percentage: int
    total_available: float
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        personal_quota = self.personal_quota

        personal_used = self.personal_used

        personal_remaining = self.personal_remaining

        personal_percentage = self.personal_percentage

        org_quota_available = self.org_quota_available

        org_percentage = self.org_percentage

        total_available = self.total_available

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "personal_quota": personal_quota,
                "personal_used": personal_used,
                "personal_remaining": personal_remaining,
                "personal_percentage": personal_percentage,
                "org_quota_available": org_quota_available,
                "org_percentage": org_percentage,
                "total_available": total_available,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        personal_quota = d.pop("personal_quota")

        personal_used = d.pop("personal_used")

        personal_remaining = d.pop("personal_remaining")

        personal_percentage = d.pop("personal_percentage")

        org_quota_available = d.pop("org_quota_available")

        org_percentage = d.pop("org_percentage")

        total_available = d.pop("total_available")

        quota_status_response = cls(
            personal_quota=personal_quota,
            personal_used=personal_used,
            personal_remaining=personal_remaining,
            personal_percentage=personal_percentage,
            org_quota_available=org_quota_available,
            org_percentage=org_percentage,
            total_available=total_available,
        )

        quota_status_response.additional_properties = d
        return quota_status_response

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
