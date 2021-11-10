# Get Ciphertext
Returns a ciphertext given the ciphertext ID.

### Arguments

|    Parameter    |  Type  | Required |                     Description                    |
|:---------------:|:------:|:--------:|:--------------------------------------------------:|
| `ciphertext_id` | string |    Yes   | The ciphertext id of the requested ciphertext info |

### Response

|   Parameter  |  Type  |          Description         |
|:------------:|:------:|:----------------------------:|
| `result` | string | The bytes of the ciphertext. |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getciphertext", "params": ["ar1k2rwg8tlg79zmgwzesyf4r3l2vyx27mtlrnjqanugx72l0jpfqxqt7sf3u"] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response

```json
{
    "jsonrpc": "2.0",
    "result": "a379350c60e4c22bd8ff75808b90cf4fa8f0ef887386f60ce1af948e0069ed06f46382f7d8a2a7f1ba1435492b5f327bb7e91d16cc4939ee6c69bc56f6264409a3a91dea4599e7ec489816cfd7e966242deecb669b13a7175ca6d49b9e6cbe0a9accda04da9532850f35ef3ae6ab13a726435fb00154774e9daea6c07b627110c9edf3b477cb2850e2734a1b0fefd1a99406e8620a0f4b1549b68f9cbb112d06161487d93570f0be62e383fdc0bc80f4c53a729f44bfc2c2f1a193e0d5cfd1071d04219124da47a6b172cd55c819b6fa5450561a2aa32b489ceecadedb02280a179dc270acc9fcdfc2d3c91765c3a01011824b878e87f9206adcd683e7b5a008e26f7a24ecf8d0ee4031fc76b5cc8025324755d2becf2279ddd1ae1b7f4ffa0350b5686dbe317b77a46e5609408fc2e4497fb003e1efd306d4a5445c742f3b04",
    "id": "1"
}
```