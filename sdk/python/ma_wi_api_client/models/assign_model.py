from collections.abc import Mapping
from typing import Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="AssignModel")


@_attrs_define
class AssignModel:
    """
    Attributes:
        model_id (str):
        modality (str):
        position (int):
        weight (int):
        rtcros_role (Union[Unset, str]):
        rtcros_task (Union[Unset, str]):
        rtcros_context (Union[Unset, str]):
        rtcros_reasoning (Union[Unset, str]):
        rtcros_output (Union[Unset, str]):
        rtcros_stop (Union[Unset, str]):
    """

    model_id: str
    modality: str
    position: int
    weight: int
    rtcros_role: Union[Unset, str] = UNSET
    rtcros_task: Union[Unset, str] = UNSET
    rtcros_context: Union[Unset, str] = UNSET
    rtcros_reasoning: Union[Unset, str] = UNSET
    rtcros_output: Union[Unset, str] = UNSET
    rtcros_stop: Union[Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        model_id = self.model_id

        modality = self.modality

        position = self.position

        weight = self.weight

        rtcros_role = self.rtcros_role

        rtcros_task = self.rtcros_task

        rtcros_context = self.rtcros_context

        rtcros_reasoning = self.rtcros_reasoning

        rtcros_output = self.rtcros_output

        rtcros_stop = self.rtcros_stop

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "model_id": model_id,
                "modality": modality,
                "position": position,
                "weight": weight,
            }
        )
        if rtcros_role is not UNSET:
            field_dict["rtcros_role"] = rtcros_role
        if rtcros_task is not UNSET:
            field_dict["rtcros_task"] = rtcros_task
        if rtcros_context is not UNSET:
            field_dict["rtcros_context"] = rtcros_context
        if rtcros_reasoning is not UNSET:
            field_dict["rtcros_reasoning"] = rtcros_reasoning
        if rtcros_output is not UNSET:
            field_dict["rtcros_output"] = rtcros_output
        if rtcros_stop is not UNSET:
            field_dict["rtcros_stop"] = rtcros_stop

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        model_id = d.pop("model_id")

        modality = d.pop("modality")

        position = d.pop("position")

        weight = d.pop("weight")

        rtcros_role = d.pop("rtcros_role", UNSET)

        rtcros_task = d.pop("rtcros_task", UNSET)

        rtcros_context = d.pop("rtcros_context", UNSET)

        rtcros_reasoning = d.pop("rtcros_reasoning", UNSET)

        rtcros_output = d.pop("rtcros_output", UNSET)

        rtcros_stop = d.pop("rtcros_stop", UNSET)

        assign_model = cls(
            model_id=model_id,
            modality=modality,
            position=position,
            weight=weight,
            rtcros_role=rtcros_role,
            rtcros_task=rtcros_task,
            rtcros_context=rtcros_context,
            rtcros_reasoning=rtcros_reasoning,
            rtcros_output=rtcros_output,
            rtcros_stop=rtcros_stop,
        )

        assign_model.additional_properties = d
        return assign_model

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
