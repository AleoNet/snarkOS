## sendtransaction

Send raw transaction bytes to this node to be added into the mempool. If valid, the transaction will be stored and propagated to all peers.

### Arguments

|      Parameter      |  Type  | Required |              Description             |
|:------------------- |:------:|:--------:|:------------------------------------ |
| `transaction_bytes` | string |    Yes   | The raw transaction hex to broadcast |

### Response

| Parameter |  Type  |                 Description                |
|:---------:|:------:|:------------------------------------------ |
| `result`  | string | The transaction id of the sent transaction |

### Example
```
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "sendtransaction", "params": ["transaction_hexstring"] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
