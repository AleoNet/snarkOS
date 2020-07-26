## getconnectioncount

Returns the number of connected peers this node has.

### Arguments

None

### Response

| Parameter |  Type  |          Description          |
|:---------:|:------:|:----------------------------- |
| `result`  | number | The number of connected nodes |

### Example
```
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getconnectioncount", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
