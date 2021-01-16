A handshake request for a `Verack` to establish a connection with a potential peer.

### Message Name

`version`

### Payload

|      Parameter     | Type   |                  Description                 |
|:------------------:|--------|:--------------------------------------------:|
| `version`          | number | The serialized bytes of a transaction        |
| `height`           | number | Latest block height of the node              |
| `nonce`            | number | Random nonce to identify the version message |
| `listening_port`   | number | The node's listening port                    |
| `timestamp`        | number | Message timestamp                            |
