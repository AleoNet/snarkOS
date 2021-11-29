# Latest Block Height
Returns the number of blocks in the canonical chain.

### Arguments

None

### Response

| Parameter |  Type  |                  Description                 |
|:---------:|:------:|:--------------------------------------------:|
| `result`  | number | The number of blocks in the canonical chain. |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "latestblockheight", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response
```json
{
   "jsonrpc":"2.0",
   "result":0,
   "id":"1"
}
```