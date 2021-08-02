Returns information about a transaction from a transaction id.

### Arguments

|     Parameter    |  Type  | Required |                      Description                     |
|:---------------- |:------:|:--------:|:---------------------------------------------------- |
| `transaction_id` | string |    Yes   | The transaction id of the requested transaction info |

### Response

|        Parameter        |  Type  |                Description               |
|:-----------------------:|:------:|:---------------------------------------- |
| `txid`                  | string | The transaction id                       |
| `size`                  | number | The size of the transaction in bytes     |
| `serial_numbers`        | array  | The list of record serial numbers        |
| `commitments`           | array  | The list of record commitments           |
| `memo`                  | string | The transaction memo                     |
| `network_id`            | number | The transaction network id               |
| `digest`                | string | The merkle tree digest                   |
| `proof`                 | string | The transaction zero knowledge proof     |
| `program_commitment`    | string | The program verification key commitment  |
| `local_data_root`       | string | The local data root                      |
| `value_balance`         | number | The transaction value balance            |
| `signatures`            | array  | The list of transaction signatures       |
| `encrypted_records`     | array  | The list of new encrypted records        |
| `transaction_metadata`  | object | The transaction metadata                 |

### Example
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "gettransactioninfo", "params": ["83fc73b8a104d7cdabe514ec4ddfeb7fd6284ff8e0a757d25d8479ed0ffe608b"] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
