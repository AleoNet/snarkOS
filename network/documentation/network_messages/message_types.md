All messages are serialized into bytes when sent and deserialized into a message payload when received.

## Block

Send a block to a peer.

#### Payload

The serialized bytes of the block.

## GetBlock

A request for a block with the specified hash.

#### Payload

|   Parameter  | Type  |              Description              |
|:------------:|-------|:-------------------------------------:|
| `block_hash` | bytes | The block hash of the requested block |

## GetMemoryPool

A request for a peer's memory pool transactions.

#### Payload

None

## GetPeers

A request for a list of the peer's connected peer addresses.

#### Payload

None

## GetSync

A request for novel block hashes.

#### Payload

|        Parameter       | Type  |                            Description                            |
|:----------------------:|-------|:-----------------------------------------------------------------:|
| `block_locator_hashes` | array | A list block hashes describing the state of the requester's chain |

## MemoryPool

A response to a GetMemoryPool request.

#### Payload

|    Parameter   | Type  |                  Description                  |
|:--------------:|-------|:---------------------------------------------:|
| `transactions` | array | A list of serialized memory pool transactions |

## Peers

A response to a GetPeers request.

#### Payload

|  Parameter  | Type  |                     Description                     |
|:-----------:|-------|:---------------------------------------------------:|
| `addresses` | array | A list of connected peers and their last seen dates |
## Ping

A ping protocol request for a Pong.

#### Payload

| Parameter | Type   |            Description            |
|:---------:|--------|:---------------------------------:|
| `nonce`   | number | A unique ping protocol identifier |

## Pong

A response to a Ping request.

#### Payload

| Parameter | Type   |              Description              |
|:---------:|--------|:-------------------------------------:|
| `nonce`   | number | The received ping protocol identifier |

## Sync

A response to a GetSync message.

#### Payload

|    Parameter   | Type  |                     Description                    |
|:--------------:|-------|:--------------------------------------------------:|
| `block_hashes` | array | A list of block hashes to share with the requester |

## SyncBlock

A response to a GetBlock request.

#### Payload

| Parameter | Type  |                 Description                 |
|:---------:|-------|:-------------------------------------------:|
| `data`    | bytes | The serialized bytes of the requested block |

## Transaction

Send a transaction to a peer.

#### Payload

| Parameter | Type  |              Description              |
|:---------:|-------|:-------------------------------------:|
| `data`    | bytes | The serialized bytes of a transaction |

## Verack

A handshake response to a Version message.

#### Payload

|      Parameter     |  Type  |           Description          |
|:------------------:|:------:|:------------------------------:|
|       `nonce`      | number | Nonce of the `Version` message |
| `address_receiver` | string |   IP of the message receiver   |
|  `address_sender`  | string |    IP of the message sender    |

## Version

A handshake request to esablish a connection with a potential peer.

#### Payload

|      Parameter     | Type   |                  Description                 |
|:------------------:|--------|:--------------------------------------------:|
| `version`          | number | The serialized bytes of a transaction        |
| `height`           | number | Latest block height of the node              |
| `nonce`            | number | Random nonce to identify the version message |
| `timestamp`        | number | Message timestamp                            |
| `address_receiver` | string | IP of the message receiver                   |
| `address_sender`   | string | IP of the message sender                     |
