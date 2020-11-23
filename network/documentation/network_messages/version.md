A handshake request for a `Verack` to establish a connection with a potential peer.

### Message Name

`version`

### Payload

|      Parameter     | Type   |                  Description                 |
|:------------------:|--------|:--------------------------------------------:|
| `version`          | number | The serialized bytes of a transaction        |
| `height`           | number | Latest block height of the node              |
| `nonce`            | number | Random nonce to identify the version message |
| `timestamp`        | number | Message timestamp                            |
| `address_receiver` | string | IP of the message receiver                   |
| `address_sender`   | string | IP of the message sender                     |
