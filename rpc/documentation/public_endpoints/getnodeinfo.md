Returns information about the node.

### Arguments

None

### Response

|     Parameter    |     Type      |                  Description                  |
|:----------------:|:-------------:|:---------------------------------------------:|
| `is_bootnode`    | bool          | Flag indicating if the node is a bootnode     |
| `is_miner`       | bool          | Flag indicating if the node is a miner        |
| `is_syncing`     | bool          | Flag indicating if the node currently syncing |
| `launched`       | timestamp     | The timestamp of when the node was launched   |
| `listening_addr` | SocketAddr    | The configured listening address of the node  |
| `version`        | string        | The version of the client binary              |
| `min_peers`      | number        | The minimum desired number of connected peers |
| `max_peers`      | number        | The maximum allowed number of connected peers |

### Example
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getnodeinfo", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
