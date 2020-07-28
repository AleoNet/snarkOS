Returns the hex encoded bytes of a record from its record commitment.

### Protected Endpoint

Yes

### Arguments

|      Parameter      |  Type  | Required |      Description      |
|:-------------------:|:------:|:--------:|:--------------------- |
| `record_commitment` | string |    Yes   | The record commitment |

### Response

| Parameter |  Type  |          Description         |
|:---------:|:------:|:---------------------------- |
| `result`  | string | The hex-encoded record bytes |

### Example
```ignore
curl --user username:password --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getrawrecord", "params": ["86be61d5f3bd795e31615d6834efefca01ad023d57c0383e2231e094bcabfc05"] }' -H 'content-type: application/json' http://127.0.0.1:3030/ 
```
