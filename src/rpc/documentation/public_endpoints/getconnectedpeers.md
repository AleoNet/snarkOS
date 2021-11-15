# Get Connected Peers
Returns the IP addresses of all connected peers.

### Arguments

None

### Response

| Parameter |  Type  |              Description                |
|:---------:|:------:|:---------------------------------------:|
| `result`  | array | An array containing the IP addresses of all connected peers. |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getconnectedpeers", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response 
```json
{
  "jsonrpc": "2.0",
  "result": [
    "111.222.111.222:4132",
    "222.111.222.111:4132",
    "111.111.222.222:4132"
  ],
  "id": "1"
}
```