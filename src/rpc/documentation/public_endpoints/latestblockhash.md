# Latest Block Hash
Returns the block hash from the head of the canonical chain.

### Arguments

None

### Response

| Parameter |  Type  |                    Description                     |
|:---------:|:------:|:--------------------------------------------------:|
| `result`  | string | The block hash of the head of the canonical chain. |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "latestblockhash", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response
```json
{
   "jsonrpc":"2.0",
   "result":"ab18946qsq2ppqylhk03ftpg7wjuknp4gwpqz0hhp8hl2ahn94sg5zqxd8qw8",
   "id":"1"
}
```