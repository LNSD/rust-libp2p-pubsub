# Protocol Documentation
<a name="top"></a>

## Table of Contents

- [libp2p/pubsub/v1/pubsub.proto](#libp2p_pubsub_v1_pubsub-proto)
    - [ControlGraft](#libp2p-pubsub-v1-ControlGraft)
    - [ControlIHave](#libp2p-pubsub-v1-ControlIHave)
    - [ControlIWant](#libp2p-pubsub-v1-ControlIWant)
    - [ControlMessage](#libp2p-pubsub-v1-ControlMessage)
    - [ControlPrune](#libp2p-pubsub-v1-ControlPrune)
    - [Frame](#libp2p-pubsub-v1-Frame)
    - [Message](#libp2p-pubsub-v1-Message)
    - [PeerInfo](#libp2p-pubsub-v1-PeerInfo)
    - [SubOpts](#libp2p-pubsub-v1-SubOpts)
  
- [Scalar Value Types](#scalar-value-types)



<a name="libp2p_pubsub_v1_pubsub-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## libp2p/pubsub/v1/pubsub.proto



<a name="libp2p-pubsub-v1-ControlGraft"></a>

### ControlGraft
The `ControlGraft` message is grafts a new link in a topic mesh.

The `ControlGraft` message informs a peer that it has been added to the local router&#39;s mesh view for the included
topic id.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| topic_id | [string](#string) | optional | The `topic_id` field specifies the topic that the message is grafting to. |






<a name="libp2p-pubsub-v1-ControlIHave"></a>

### ControlIHave
The `ControlIHave` message is used to advertise messages that a peer has.

It provides the remote peer with a list of messages that were recently seen by the local router. The remote peer
may then request the full message content with a `ControlIWant` message.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| topic_id | [string](#string) | optional | The `topic_id` field specifies the topic that the message IDs are for. |
| message_ids | [bytes](#bytes) | repeated | The `message_ids` field contains a list of message IDs that the local peer has. |






<a name="libp2p-pubsub-v1-ControlIWant"></a>

### ControlIWant
The `ControlIWant` message is used to request messages from a peer.

It provides the remote peer with a list of messages that the local peer is interested in. The requested messages IDs
are those that were previously announced by the remote peer in an `ControlIHave` message. The remote peer may then
send the full message content with a `Message` message.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| message_ids | [bytes](#bytes) | repeated | The `message_ids` field contains a list of message IDs that the local peer is interested in. |






<a name="libp2p-pubsub-v1-ControlMessage"></a>

### ControlMessage
The `ControlMessage` message is used to send control messages between peers.

It contains one or more control messages.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| ihave | [ControlIHave](#libp2p-pubsub-v1-ControlIHave) | repeated | The `ihave` field contains a list of `ControlIHave` messages. |
| iwant | [ControlIWant](#libp2p-pubsub-v1-ControlIWant) | repeated | The `iwant` field contains a list of `ControlIWant` messages. |
| graft | [ControlGraft](#libp2p-pubsub-v1-ControlGraft) | repeated | The `graft` field contains a list of `ControlGraft` messages. |
| prune | [ControlPrune](#libp2p-pubsub-v1-ControlPrune) | repeated | The `prune` field contains a list of `ControlPrune` messages. |






<a name="libp2p-pubsub-v1-ControlPrune"></a>

### ControlPrune
The `ControlPrune` message is used to prune a link from a topic mesh.

The `ControlPrune` message informs a peer that it has been removed from the local router&#39;s mesh view for the
included topic id.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| topic_id | [string](#string) | optional | The `topic_id` field specifies the topic that the message is pruning from. |
| peers | [PeerInfo](#libp2p-pubsub-v1-PeerInfo) | repeated | The `peers` field contains a list of `PeerInfo` messages.

It is part of the gossipsub v1.1 Peer eXchange (PX) protocol extension.

gossipsub v1.1 PX |
| backoff | [uint64](#uint64) | optional | The `backoff` field specifies the time (in seconds) that the remote peer should wait before attempting to re-graft. |






<a name="libp2p-pubsub-v1-Frame"></a>

### Frame
Communication between peers happens in the form of exchanging protobuf `Frame` messages between participating
peers.

The `Frame` message is a relatively simple protobuf message containing zero or more subscription action messages,
zero or more data messages, or zero or more control messages.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| subscriptions | [SubOpts](#libp2p-pubsub-v1-SubOpts) | repeated | Subscription action messages. |
| publish | [Message](#libp2p-pubsub-v1-Message) | repeated | Data messages. |
| control | [ControlMessage](#libp2p-pubsub-v1-ControlMessage) | optional | Control messages. |






<a name="libp2p-pubsub-v1-Message"></a>

### Message



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| from | [bytes](#bytes) | optional | The `from` field (optional) denotes the author of the message.

This is the peer who initially authored the message, and NOT the peer who propagated it. Thus, as the message is routed through a swarm of pubsubbing peers, the original authorship is preserved. |
| data | [bytes](#bytes) | optional | The `data` field (optional) contains the payload of the message.

This is an opaque byte array, whose contents are not interpreted by pubsub. The maximum size of a message is determined by the pubsub implementation. |
| seqno | [bytes](#bytes) | optional | The `seqno` field (optional) contains a sequence number for the message.

No two messages on a pubsub topic from the same peer have the same `seqno` value, however messages from different peers may have the same sequence number. In other words, this number is not globally unique.

This is an opaque byte array, whose contents are not interpreted by pubsub. The maximum size of a sequence number is determined by the pubsub implementation. |
| topic | [string](#string) |  | The `topic` field specifies the topic that the message should be published to. |
| signature | [bytes](#bytes) | optional | The `signature` field (optional) contains a signature of the message.

This is an opaque byte array, whose contents are not interpreted by pubsub. The maximum size of a signature is determined by the pubsub implementation. |
| key | [bytes](#bytes) | optional | The `key` field (optional) contains a public key that can be used to verify the signature.

This is an opaque byte array, whose contents are not interpreted by pubsub. The maximum size of a key is determined by the pubsub implementation. |






<a name="libp2p-pubsub-v1-PeerInfo"></a>

### PeerInfo
The `PeerInfo` message is used to provide information about a peer.

It contains the peer&#39;s ID and a signed peer record.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| peer_id | [bytes](#bytes) | optional |  |
| signed_peer_record | [bytes](#bytes) | optional |  |






<a name="libp2p-pubsub-v1-SubOpts"></a>

### SubOpts
The `SubOpts` message is used to subscribe or unsubscribe from a topic.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| subscribe | [bool](#bool) | optional | The `subscribe` field indicates whether the message is a subscription or unsubscription. |
| topic_id | [string](#string) | optional | The `topic_id` field specifies the topic that the message is subscribing or unsubscribing from. |





 

 

 

 



## Scalar Value Types

| .proto Type | Notes | C++ | Java | Python | Go | C# | PHP | Ruby |
| ----------- | ----- | --- | ---- | ------ | -- | -- | --- | ---- |
| <a name="double" /> double |  | double | double | float | float64 | double | float | Float |
| <a name="float" /> float |  | float | float | float | float32 | float | float | Float |
| <a name="int32" /> int32 | Uses variable-length encoding. Inefficient for encoding negative numbers – if your field is likely to have negative values, use sint32 instead. | int32 | int | int | int32 | int | integer | Bignum or Fixnum (as required) |
| <a name="int64" /> int64 | Uses variable-length encoding. Inefficient for encoding negative numbers – if your field is likely to have negative values, use sint64 instead. | int64 | long | int/long | int64 | long | integer/string | Bignum |
| <a name="uint32" /> uint32 | Uses variable-length encoding. | uint32 | int | int/long | uint32 | uint | integer | Bignum or Fixnum (as required) |
| <a name="uint64" /> uint64 | Uses variable-length encoding. | uint64 | long | int/long | uint64 | ulong | integer/string | Bignum or Fixnum (as required) |
| <a name="sint32" /> sint32 | Uses variable-length encoding. Signed int value. These more efficiently encode negative numbers than regular int32s. | int32 | int | int | int32 | int | integer | Bignum or Fixnum (as required) |
| <a name="sint64" /> sint64 | Uses variable-length encoding. Signed int value. These more efficiently encode negative numbers than regular int64s. | int64 | long | int/long | int64 | long | integer/string | Bignum |
| <a name="fixed32" /> fixed32 | Always four bytes. More efficient than uint32 if values are often greater than 2^28. | uint32 | int | int | uint32 | uint | integer | Bignum or Fixnum (as required) |
| <a name="fixed64" /> fixed64 | Always eight bytes. More efficient than uint64 if values are often greater than 2^56. | uint64 | long | int/long | uint64 | ulong | integer/string | Bignum |
| <a name="sfixed32" /> sfixed32 | Always four bytes. | int32 | int | int | int32 | int | integer | Bignum or Fixnum (as required) |
| <a name="sfixed64" /> sfixed64 | Always eight bytes. | int64 | long | int/long | int64 | long | integer/string | Bignum |
| <a name="bool" /> bool |  | bool | boolean | boolean | bool | bool | boolean | TrueClass/FalseClass |
| <a name="string" /> string | A string must always contain UTF-8 encoded or 7-bit ASCII text. | string | String | str/unicode | string | string | string | String (UTF-8) |
| <a name="bytes" /> bytes | May contain any arbitrary sequence of bytes. | string | ByteString | str | []byte | ByteString | string | String (ASCII-8BIT) |

