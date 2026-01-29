from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.chat_choice import ChatChoice
    from ..models.routing_metadata import RoutingMetadata
    from ..models.token_usage import TokenUsage


T = TypeVar("T", bound="UnifiedChatResponse")


@_attrs_define
class UnifiedChatResponse:
    """
    Attributes:
        id (str):
        object_ (str):
        created (int):
        model (str):
        choices (list['ChatChoice']):
        usage (Union[Unset, TokenUsage]):
        routing_metadata (Union[Unset, RoutingMetadata]):
    """

    id: str
    object_: str
    created: int
    model: str
    choices: list["ChatChoice"]
    usage: Union[Unset, "TokenUsage"] = UNSET
    routing_metadata: Union[Unset, "RoutingMetadata"] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        id = self.id

        object_ = self.object_

        created = self.created

        model = self.model

        choices = []
        for choices_item_data in self.choices:
            choices_item = choices_item_data.to_dict()
            choices.append(choices_item)

        usage: Union[Unset, dict[str, Any]] = UNSET
        if not isinstance(self.usage, Unset):
            usage = self.usage.to_dict()

        routing_metadata: Union[Unset, dict[str, Any]] = UNSET
        if not isinstance(self.routing_metadata, Unset):
            routing_metadata = self.routing_metadata.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "id": id,
                "object": object_,
                "created": created,
                "model": model,
                "choices": choices,
            }
        )
        if usage is not UNSET:
            field_dict["usage"] = usage
        if routing_metadata is not UNSET:
            field_dict["routing_metadata"] = routing_metadata

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.chat_choice import ChatChoice
        from ..models.routing_metadata import RoutingMetadata
        from ..models.token_usage import TokenUsage

        d = dict(src_dict)
        id = d.pop("id")

        object_ = d.pop("object")

        created = d.pop("created")

        model = d.pop("model")

        choices = []
        _choices = d.pop("choices")
        for choices_item_data in _choices:
            choices_item = ChatChoice.from_dict(choices_item_data)

            choices.append(choices_item)

        _usage = d.pop("usage", UNSET)
        usage: Union[Unset, TokenUsage]
        if isinstance(_usage, Unset):
            usage = UNSET
        else:
            usage = TokenUsage.from_dict(_usage)

        _routing_metadata = d.pop("routing_metadata", UNSET)
        routing_metadata: Union[Unset, RoutingMetadata]
        if isinstance(_routing_metadata, Unset):
            routing_metadata = UNSET
        else:
            routing_metadata = RoutingMetadata.from_dict(_routing_metadata)

        unified_chat_response = cls(
            id=id,
            object_=object_,
            created=created,
            model=model,
            choices=choices,
            usage=usage,
            routing_metadata=routing_metadata,
        )

        unified_chat_response.additional_properties = d
        return unified_chat_response

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
