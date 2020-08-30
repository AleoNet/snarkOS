A handshake response to a `Version` message.

### Message Name

`verack`

### Payload

|      Parameter     |  Type  |           Description          |
|:------------------:|:------:|:------------------------------:|
|       `nonce`      | number | Nonce of the `Version` message |
| `address_receiver` | string |   IP of the message receiver   |
|  `address_sender`  | string |    IP of the message sender    |
