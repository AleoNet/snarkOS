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
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getciphertext", "params": ["ar18gr9hxzr40ve9238eddus8vq7ka8a07wk63666hmdqk7ess5mqqsh5xazm"] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response

```json
{
  "jsonrpc": "2.0",
  "result": "580a3b830d17818b866c8f4f7bc4f518dcd775adce1274e902d07edc49e3e9005418da88939bac4437a271cb34cf792e3bad3385fe05ec086b5e5c485b291209a7ced29b1f6f169dd13504a71ce57c3348fe8778c7a102e26791fb298a7a0a0edfae8e065599cd3869d08a91ffa06fb6e41f68033d9b8d9269fa6f948f167f07c7d57ab6f3198c3d568a60525cb4959df7655dc8de9751d3cd8ec6b5aa834c0b0b081c6734fb62670b80cc885210392e6f56cd2b4aec22c53257a3422fe474041540a950c102b00c36cb6e889a7cefe2822a003e9332f3536a18fd3bc1690e11094593d25543c3df7e2fc4a24754eafdf4e0002ed4673c1a84c49ddcc510870efef1b4f95f0cf7d1a2e0e754eaa188a22cefb4938e616506154135091874c10ed4abe8bf94915a782e545acc07663237c2c826fd2c516fd14a2f9d2c570afe0d",
  "id": "1"
}
```