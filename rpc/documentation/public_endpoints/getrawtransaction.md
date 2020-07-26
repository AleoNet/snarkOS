### getrawtransaction

Returns hex encoded bytes of a transaction from its transaction id.

#### Arguments

|     Parameter    |  Type  | Required |                     Description                     |
|:---------------- |:------:|:--------:|:--------------------------------------------------- |
| `transaction_id` | string |    Yes   | The transaction id of the requested transaction hex |

#### Response

| Parameter |  Type  |            Description            |
|:---------:|:------:|:---------------------------------:|
| `result`  | string | The hex-encoded transaction bytes |

#### Example
```
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getrawtransaction", "params": ["83fc73b8a104d7cdabe514ec4ddfeb7fd6284ff8e0a757d25d8479ed0ffe608b"] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
