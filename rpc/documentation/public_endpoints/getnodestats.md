Returns statistics related to the node.

### Arguments

None

### Response

|            Parameter           | Type |                         Description                       |
|:------------------------------:|:----:|:---------------------------------------------------------:|
| `send_success_count`           | u64  | The number of successfully sent messages                  |
| `send_failure_count`           | u64  | The number of failures to send messages                   |
| `recv_success_count`           | u64  | The number of successfully processed inbound messages     |
| `recv_failure_count`           | u64  | The number of inbound messages that couldn't be processed |
| `inbound_channel_items`        | u64  | The number of inbound items queued to be processed        |
| `inbound_connection_requests`  | u64  | The number of connection requests the node has received   |
| `outbound_connection_requests` | u64  | The number of connection requests the node has made       |
| `number_of_connected_peers`    | u16  | The number of currently connected peers                   |
| `number_of_connecting_peers`   | u16  | The number of currently connecting peers                  |
| `blocks_mined`                 | u32  | The number of blocks the node has mined                   |
| `block_height`                 | u32  | The current block height of the node                      |
| `recv_blocks`                  | u64  | The number of all received Block messages                 |
| `recv_getmemorypool`           | u64  | The number of all received GetMemoryPool messages         |
| `recv_getpeers`                | u64  | The number of all received GetPeers messages              |
| `recv_getsync`                 | u64  | The number of all received GetSync messages               |
| `recv_memorypool`              | u64  | The number of all received MemoryPool messages            |
| `recv_peers`                   | u64  | The number of all received Peers messages                 |
| `recv_pings`                   | u64  | The number of all received Ping messages                  |
| `recv_pongs`                   | u64  | The number of all received Pong messages                  |
| `recv_syncs`                   | u64  | The number of all received Sync messages                  |
| `recv_syncblocks`              | u64  | The number of all received SyncBlock messages             |
| `recv_transactions`            | u64  | The number of all received Transaction messages           |
| `recv_unknown`                 | u64  | The number of all received Unknown messages               |

### Example
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getnodestats", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
