Validate and return if the transaction is valid.

### Arguments

|      Parameter      |  Type  | Required |             Description             |
|:------------------- |:------:|:--------:|:----------------------------------- |
| `transaction_bytes` | string |    Yes   | The raw transaction hex to validate |

### Response

| Parameter |   Type  |             Description             |
|:---------:|:-------:|:----------------------------------- |
| `result`  | boolean | Check that the transaction is valid |

### Example
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "validaterawtransaction", "params": ["transaction_hexstring"] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
