## fetchrecordcommitments

Return the node's stored record commitments.

### Arguments

`None`

### Response

| Parameter |  Type |             Description            |
|:---------:|:-----:|:---------------------------------- |
| `result`  | array | The list stored record commitments |

### Example
```
curl --user username:password --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "fetchrecordcommitments", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/ 
```
