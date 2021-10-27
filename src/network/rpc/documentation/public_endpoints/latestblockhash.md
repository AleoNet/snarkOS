Returns the block hash from the head of the canonical chain.

### Arguments

None

### Response

| Parameter |  Type  |                    Description                     |
|:---------:|:------:|:--------------------------------------------------:|
| `result`  | string | The block hash of the head of the canonical chain. |

### Example
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "latestblockhash", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
