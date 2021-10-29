Returns a ciphertext given the ciphertext ID.

### Arguments

|    Parameter    |  Type  | Required |                     Description                    |
|:---------------:|:------:|:--------:|:--------------------------------------------------:|
| `ciphertext_id` | string |    Yes   | The ciphertext id of the requested ciphertext info |

### Response

|   Parameter  |  Type  |          Description         |
|:------------:|:------:|:----------------------------:|
| `ciphertext` | string | The bytes of the ciphertext. |

### Example
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getciphertext", "params": ["83fc73b8a104d7cdabe514ec4ddfeb7fd6284ff8e0a757d25d8479ed0ffe608b"] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
