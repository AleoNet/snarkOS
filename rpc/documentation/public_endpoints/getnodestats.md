Returns statistics related to the node.

### Arguments

None

### Response

|             Parameter            | Type |                         Description                       |
|:--------------------------------:|:----:|:---------------------------------------------------------:|
| `block_height`                   | u32  | The current block height of the node                      |
| `blocks_mined`                   | u32  | The number of blocks the node has mined                   |
| `connections.all_accepted`       | u64  | The number of connection requests the node has received   |
| `connections.all_initiated`      | u64  | The number of connection requests the node has made       |
| `connections.connected_peers`    | u16  | The number of currently connected peers                   |
| `connections.connecting_peers`   | u16  | The number of currently connecting peers                  |
| `connections.disconnected_peers` | u16  | The number of known disconnected peers                    |
| `handshakes.failures_init`       | u64  | The number of failed handshakes as the initiator          |
| `handshakes.failures_resp`       | u64  | The number of failed handshakes as the responder          |
| `handshakes.successes_init`      | u64  | The number of successful handshakes as the initiator      |
| `handshakes.successes_resp`      | u64  | The number of successful handshakes as the responder      |
| `handshakes.timeouts_init`       | u64  | The number of handshake timeouts as the initiator         |
| `handshakes.timeouts_resp`       | u64  | The number of handshake timeouts as the responder         |
| `inbound.all_successes`          | u64  | The number of successfully processed inbound messages     |
| `inbound.all_failures`           | u64  | The number of inbound messages that couldn't be processed |
| `inbound.queued_messages`        | u64  | The number of inbound messages queued to be processed     |
| `inbound.blocks`                 | u64  | The number of all received Block messages                 |
| `inbound.getmemorypool`          | u64  | The number of all received GetMemoryPool messages         |
| `inbound.getpeers`               | u64  | The number of all received GetPeers messages              |
| `inbound.getsync`                | u64  | The number of all received GetSync messages               |
| `inbound.memorypool`             | u64  | The number of all received MemoryPool messages            |
| `inbound.peers`                  | u64  | The number of all received Peers messages                 |
| `inbound.pings`                  | u64  | The number of all received Ping messages                  |
| `inbound.pongs`                  | u64  | The number of all received Pong messages                  |
| `inbound.syncs`                  | u64  | The number of all received Sync messages                  |
| `inbound.syncblocks`             | u64  | The number of all received SyncBlock messages             |
| `inbound.transactions`           | u64  | The number of all received Transaction messages           |
| `inbound.unknown`                | u64  | The number of all received Unknown messages               |
| `outbound.all_successes`         | u64  | The number of successfully sent messages                  |
| `outbound.all_failures`          | u64  | The number of failures to send messages                   |

### Example
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getnodestats", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
