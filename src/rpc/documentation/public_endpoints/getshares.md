# Get Shares
Returns the amount of shares submitted to an operator in total.

### Arguments

None

### Response

| Parameter |  Type  |                       Description                       |
|:---------:|:------:|:-------------------------------------------------------:|
| `result`  |  u64   | The amount of shares submitted to the operator in total |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"1", "method": "getshares", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response
```json
{
   "jsonrpc":"2.0",
   "result":"46239",
   "id":"1"
}
```
