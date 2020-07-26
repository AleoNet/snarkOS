## createaccount

Generate a new account private key and its corresponding account address.

### Arguments

`None`

### Response

|   Parameter   |  Type  |         Description         |
|:------------- |:------:|:--------------------------- |
| `private_key` | string | An Aleo account private key |
| `address`     | string | An Aleo account address     |

### Example
```
curl --user username:password --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "createaccount", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/ 
```
