Returns information about the node.

### Arguments

None 

### Response

|   Parameter  | Type |                  Description                  |
|:------------:|:----:|:---------------------------------------------:|
|  `is_miner`  | bool | Flag indicating if the node is a miner        |
| `is_snycing` | bool | Flag indicating if the node currently syncing |

### Example
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getnodeinfo", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
