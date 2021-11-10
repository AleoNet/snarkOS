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
   "result":"at1mka6m3kfsgt5dpnfurk2ydjefqjzng4aawj7lkpc32pjkg86hyysrke9nf",
   "id":"1"
}
```