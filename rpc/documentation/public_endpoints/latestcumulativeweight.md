# Latest Cumulative Weight
Returns the latest cumulative weight of the canonical chain.

### Arguments

None

### Response

| Parameter |  Type  |                     Description                      |
|:---------:|:------:|:----------------------------------------------------:|
| `result`  | number | The latest cumulative weight of the canonical chain. |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"1", "method": "latestcumulativeweight", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response
```json
{
   "jsonrpc":"2.0",
   "result":4665,
   "id":"1"
}
```