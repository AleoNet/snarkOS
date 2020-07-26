## getbestblockhash

Returns the block hash of the head of the best valid chain.

### Arguments

None

### Response

| Parameter |  Type  |                  Description                  |
|:---------:|:------:|:---------------------------------------------:|
| `result`  | string | The block hash of the most recent valid block |

### Example
```
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getbestblockhash", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
