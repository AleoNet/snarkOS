## getblockcount

Returns the number of blocks in the best valid chain.

### Arguments

None

### Response

| Parameter |  Type  |                  Description                 |
|:---------:|:------:|:--------------------------------------------:|
| `result`  | string | The number of blocks in the best valid chain |

### Example
```
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getblockcount", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
