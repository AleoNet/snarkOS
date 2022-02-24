# Get Shares For Prover
Returns the number of shares submitted by a prover, given their address.

### Arguments

| Parameter |  Type  | Required |          Description           |
|:---------:|:------:|:--------:|:------------------------------:|
| `prover`  | string |   Yes    | The Aleo address of the prover |

### Response

| Parameter |  Type  |                 Description                  |
|:---------:|:------:|:--------------------------------------------:|
| `result`  |  u64   | The number of shares submitted by the prover |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"1", "method": "getsharesforprover", "params": ["aleo_address"] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response
```json
{
   "jsonrpc":"2.0",
   "result":"581",
   "id":"1"
}
```
