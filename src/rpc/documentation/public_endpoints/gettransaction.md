Returns a transaction given the transaction ID.

### Arguments

|     Parameter    |  Type  | Required |                   Description                   |
|:----------------:|:------:|:--------:|:-----------------------------------------------:|
| `transaction_id` | string |    Yes   | The transaction id of the requested transaction |

### Response

|        Parameter        |  Type  |                          Description                         |
|:-----------------------:|:------:|:------------------------------------------------------------:|
| `transaction_id`        | string | The ID of the transaction.                                   |
| `network_id`            | number | The ID of the network.                                       |
| `inner_circuit_id`      | string | The ID of the inner circuit used to execute the transaction. | 
| `transitions`           | array  | The list of state transitions.                               |
| `events`                | array  | The list of events emitted from the transaction.             |

### Example
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "gettransaction", "params": ["83fc73b8a104d7cdabe514ec4ddfeb7fd6284ff8e0a757d25d8479ed0ffe608b"] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
