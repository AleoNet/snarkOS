# Send Transaction
Send a transaction hex to this node to be added into the mempool.
Returns the transaction ID.
If the given transaction is valid, it is added to the memory pool and propagated to all peers.

### Arguments

|     Parameter     |  Type  | Required |              Description             |
|:-----------------:|:------:|:--------:|:------------------------------------:|
| `transaction_hex` | string |    Yes   | The raw transaction hex to broadcast |

### Response

| Parameter |  Type  |                 Description                |
|:---------:|:------:|:------------------------------------------:|
| `result`  | string | The transaction id of the sent transaction |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "sendtransaction", "params": ["transaction_hexstring"] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response
```json
{
   "jsonrpc":"2.0",
   "result":"at1pazplqjlhvyvex64xrykr4egpt77z05n74u5vlnkyv05r3ctgyxs0cgj6w",
   "id":"1"
}
```