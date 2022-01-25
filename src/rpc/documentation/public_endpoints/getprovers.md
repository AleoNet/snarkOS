# Get Provers
Returns the addresses of all provers that are registered on an operator.

### Arguments

None

### Response

| Parameter |  Type  |                Description                 |
|:---------:|:------:|:------------------------------------------:|
| `result`  | array  | All of the addresses known to the operator |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"1", "method": "getprovers", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response
```json
{
   "jsonrpc":"2.0",
   "result": ["aleo1...", "aleo1..."],
   "id":"1"
}
```
