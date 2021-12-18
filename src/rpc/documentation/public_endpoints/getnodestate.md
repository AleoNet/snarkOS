# Get Node State
Returns the current state of this node.

### Arguments

None

### Response

|             Parameter             |  Type  |                     Description                      |
|:---------------------------------:|:------:|:----------------------------------------------------:|
|         `candidate_peers`         | array  |      The list of candidate peer IPs addresses.       |
|         `connected_peers`         | array  |       The list of connected peer IP addresses.       |
|       `latest_block_height`       | number |               The latest block height.               |
|    `latest_cumulative_weight`     | number | The latest cumulative weight of the canonical chain. |
|    `number_of_candidate_peers`    | number |            The number of candidate peers.            |
|    `number_of_connected_peers`    | number |            The number of connected peers.            |
| `number_of_connected_sync_nodes`  | number |            The number of connected peers.            |
|            `software`             | string |       The rust cargo package name and version.       |
|             `status`              | string |                The state of the node.                |
|              `type`               | string |                The type of the node.                 |
|             `version`             | number |         The version of the network protocol.         |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"1", "method": "getnodestate", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```


### Example Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "candidate_peers": [
      "127.0.0.1:4136",
      "127.0.0.1:4134",
      "144.126.212.176:4132",
      "127.0.0.1:4133",
      "127.0.0.1:4135"
    ],
    "connected_peers": [
      "128.199.5.137:4132",
      "144.126.223.138:4135"
    ],
    "latest_block_height": 4000,
    "latest_cumulative_weight": "4668",
    "number_of_candidate_peers": 5,
    "number_of_connected_peers": 2,
    "number_of_connected_sync_nodes": 0,
    "software": "snarkOS 2.0.0",
    "status": "Ready",
    "type": "Client",
    "version": 10
  },
  "id": "1"
}
```