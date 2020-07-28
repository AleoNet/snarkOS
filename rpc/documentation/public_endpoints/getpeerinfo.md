Returns the node's connected peers.

### Arguments

None 

### Response

| Parameter |  Type |           Description          |
|:---------:|:-----:|:------------------------------:|
| `peers`   | array | The list of connected peer IPs |

### Example
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getpeerinfo", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
