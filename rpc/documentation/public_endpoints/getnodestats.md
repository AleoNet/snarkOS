Returns statistics related to the node.

### Arguments

None

### Response

|            Parameter           | Type |                        Description                      |
|:------------------------------:|:----:|:-------------------------------------------------------:|
| `send_success_count`           | u64  | The number of successfully sent messages                |
| `send_failure_count`           | u64  | The number of failures to send messages                 |
| `inbound_channel_items`        | u64  | The number of inbound items queued to be processed      |
| `inbound_connection_requests`  | u64  | The number of connection requests the node has received |
| `outbound_connection_requests` | u64  | The number of connection requests the node has made     |
| `number_of_connected_peers`    | u16  | The number of currently connected peers                 |
| `number_of_connecting_peers`   | u16  | The number of currently connecting peers                |
| `blocks_mined`                 | u32  | The number of blocks the node has mined                 |
| `block_height`                 | u32  | The current block height of the node                    |

### Example
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getnodestats", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
