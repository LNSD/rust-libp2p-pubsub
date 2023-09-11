# Protocol Documentation
<a name="top"></a>

## Table of Contents

- [libp2p/topic_descriptor/v1/topic_descriptor.proto](#libp2p_topic_descriptor_v1_topic_descriptor-proto)
    - [TopicDescriptor](#libp2p-topic_descriptor-v1-TopicDescriptor)
    - [TopicDescriptor.AuthOpts](#libp2p-topic_descriptor-v1-TopicDescriptor-AuthOpts)
    - [TopicDescriptor.EncOpts](#libp2p-topic_descriptor-v1-TopicDescriptor-EncOpts)
  
    - [TopicDescriptor.AuthOpts.AuthMode](#libp2p-topic_descriptor-v1-TopicDescriptor-AuthOpts-AuthMode)
    - [TopicDescriptor.EncOpts.EncMode](#libp2p-topic_descriptor-v1-TopicDescriptor-EncOpts-EncMode)
  
- [Scalar Value Types](#scalar-value-types)



<a name="libp2p_topic_descriptor_v1_topic_descriptor-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## libp2p/topic_descriptor/v1/topic_descriptor.proto



<a name="libp2p-topic_descriptor-v1-TopicDescriptor"></a>

### TopicDescriptor



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| name | [string](#string) | optional |  |
| auth | [TopicDescriptor.AuthOpts](#libp2p-topic_descriptor-v1-TopicDescriptor-AuthOpts) | optional |  |
| enc | [TopicDescriptor.EncOpts](#libp2p-topic_descriptor-v1-TopicDescriptor-EncOpts) | optional |  |






<a name="libp2p-topic_descriptor-v1-TopicDescriptor-AuthOpts"></a>

### TopicDescriptor.AuthOpts



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| mode | [TopicDescriptor.AuthOpts.AuthMode](#libp2p-topic_descriptor-v1-TopicDescriptor-AuthOpts-AuthMode) | optional |  |
| keys | [bytes](#bytes) | repeated | root keys to trust |






<a name="libp2p-topic_descriptor-v1-TopicDescriptor-EncOpts"></a>

### TopicDescriptor.EncOpts



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| mode | [TopicDescriptor.EncOpts.EncMode](#libp2p-topic_descriptor-v1-TopicDescriptor-EncOpts-EncMode) | optional |  |
| key_hashes | [bytes](#bytes) | repeated | the hashes of the shared keys used (salted) |





 


<a name="libp2p-topic_descriptor-v1-TopicDescriptor-AuthOpts-AuthMode"></a>

### TopicDescriptor.AuthOpts.AuthMode


| Name | Number | Description |
| ---- | ------ | ----------- |
| NONE | 0 | no authentication, anyone can publish |
| KEY | 1 | only messages signed by keys in the topic descriptor are accepted |
| WOT | 2 | web of trust, certificates can allow publisher set to grow |



<a name="libp2p-topic_descriptor-v1-TopicDescriptor-EncOpts-EncMode"></a>

### TopicDescriptor.EncOpts.EncMode


| Name | Number | Description |
| ---- | ------ | ----------- |
| NONE | 0 | no encryption, anyone can read |
| SHAREDKEY | 1 | messages are encrypted with shared key |
| WOT | 2 | web of trust, certificates can allow publisher set to grow |


 

 

 



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

