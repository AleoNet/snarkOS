Decrypts the record ciphertext and returns the hex encoded bytes of the record.

### Arguments

|      Parameter      |  Type  | Required |                     Description                     |
|:-------------------:|:------:|:--------:|:---------------------------------------------------:|
| `record_ciphertext` | string |    Yes   |                 The encrypted record                |
|  `account_view_key` | string |    Yes   | The account view key used to decrypt the ciphertext |

### Response

| Parameter |  Type  |          Description         |
|:---------:|:------:|:---------------------------- |
| `result`  | string | The hex-encoded record bytes |


### Example
```
curl --user username:password --data-binary '{ 
    "jsonrpc":"2.0",
    "id": "1",
    "method": "decryptrecord",
    "params": [
       {
        "record_ciphertext": "record_ciphertext_string",
        "account_view_key": "account_view_key_string"
       }
    ]
}' -H 'content-type: application/json' http://127.0.0.1:3030/
```