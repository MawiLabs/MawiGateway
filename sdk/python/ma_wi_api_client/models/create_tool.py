from collections.abc import Mapping
from typing import Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="CreateTool")


@_attrs_define
class CreateTool:
    """Request to create a tool for an agentic service

    Attributes:
        name (str):
        description (str):
        tool_type (str):
        target_id (str):
        position (int):
        parameters_schema (Union[Unset, Any]):
    """

    name: str
    description: str
    tool_type: str
    target_id: str
    position: int
    parameters_schema: Union[Unset, Any] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        name = self.name

        description = self.description

        tool_type = self.tool_type

        target_id = self.target_id

        position = self.position

        parameters_schema = self.parameters_schema

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "name": name,
                "description": description,
                "tool_type": tool_type,
                "target_id": target_id,
                "position": position,
            }
        )
        if parameters_schema is not UNSET:
            field_dict["parameters_schema"] = parameters_schema

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        name = d.pop("name")

        description = d.pop("description")

        tool_type = d.pop("tool_type")

        target_id = d.pop("target_id")

        position = d.pop("position")

        parameters_schema = d.pop("parameters_schema", UNSET)

        create_tool = cls(
            name=name,
            description=description,
            tool_type=tool_type,
            target_id=target_id,
            position=position,
            parameters_schema=parameters_schema,
        )

        create_tool.additional_properties = d
        return create_tool

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
