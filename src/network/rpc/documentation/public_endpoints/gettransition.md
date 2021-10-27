Returns a transition given the transition ID.

### Arguments

|    Parameter    |  Type  | Required |                     Description                    |
|:--------------- |:------:|:--------:|:--------------------------------------------------:|
| `transition_id` | string |    Yes   | The transition id of the requested transition info |

### Response

|         Parameter        |  Type  |                                   Description                                   |
|:------------------------:|:------:|:-------------------------------------------------------------------------------:|
| `transition_id`          | string | The ID of the transition.                                                       |
| `block_hash`             | string | The The block hash used to prove inclusion of ledger-consumed records.          | 
| `local_commitments_root` | string | The local commitments root used to prove inclusion of locally-consumed records. |
| `serial_numbers`         | array  | The serial numbers of the input records.                                        |
| `commitments`            | array  | The commitments of the output records.                                          |
| `ciphertexts`            | array  | The ciphertexts of the output records.                                          |
| `value_balance`          | number | The difference between the input and output record values.                      |
| `proof`                  | string | The zero-knowledge proof attesting to the validity of the transition.           |

### Example
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "gettransition", "params": ["83fc73b8a104d7cdabe514ec4ddfeb7fd6284ff8e0a757d25d8479ed0ffe608b"] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
