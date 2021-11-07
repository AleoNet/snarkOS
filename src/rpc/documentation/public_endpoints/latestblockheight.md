Returns the number of blocks in the canonical chain.

### Arguments

None

### Response

| Parameter |  Type  |                  Description                 |
|:---------:|:------:|:--------------------------------------------:|
| `result`  | number | The number of blocks in the canonical chain. |

### Example
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "latestblockheight", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
