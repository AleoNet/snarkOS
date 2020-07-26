### createaccount

Generate a new account private key and public key pair.

#### Arguments

`None`

#### Response

|   Parameter   |  Type  |         Description         |
|:------------- |:------:|:--------------------------- |
| `private_key` | string | An Aleo account private key |
| `public_key`  | string | An Aleo account public key  |

#### Example
```
curl --user username:password --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "createaccount", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/ 
```
